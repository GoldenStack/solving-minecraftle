use std::{collections::HashMap, fs::{read_dir, read_to_string, DirEntry}, io, path::PathBuf, str::FromStr, vec};

use anyhow::{anyhow, Context, Result};
use itertools::Itertools;
use serde_json::Value;

use crate::{Ingredient, Material, Recipe, TAG_DIRECTORY};

pub fn list_dir(path: &str) -> Result<Vec<PathBuf>> {
    let dir = read_dir(path)
        .with_context(|| format!("while trying to list directory {path}"))?;

    let names = dir.into_iter().collect::<io::Result<Vec<DirEntry>>>()
        .with_context(|| format!("while trying to parse item in directory list {path}"))?;

    Ok(names.into_iter().map(|dir| dir.path()).collect_vec())
}

/// Reads JSON from a path, trying to return the parsed value;
pub fn read_json(path: &PathBuf) -> Result<Value> {
    let string = read_to_string(path)
        .with_context(|| format!("while trying to parse path {path:?}"))?;

    serde_json::from_str::<Value>(&string)
        .with_context(|| format!("while trying to parse json from path {path:?}"))
}

/// Determines whether or not a recipe should be parsed in the first place.
pub fn filter_recipe(json: &Value) -> bool {
    let Some(Value::String(category)) = json.get("type") else {
        return false;
    };

    category == "minecraft:crafting_shaped" || category == "minecraft:crafting_shapeless"
}

/// Tries to parse a recipe from the provided JSON.
pub fn parse_recipe(json: Value) -> Result<(String, Recipe)> {
    let Some(Value::String(category)) = json.get("type") else {
        return Err(anyhow!("expected string category at path type'"));
    };

    fn parse_shaped(json: &Value) -> Result<Recipe> {
        let Some(Value::Object(object)) = json.get("key") else {
            return Err(anyhow!("expected object at path 'key'"));
        };

        let key = object.iter()
            .map(|(k, v)| parse_ingredient(v).map(|v| (k, v)))
            .collect::<Result<HashMap<_, _>, _>>()
            .with_context(|| "while parsing ingredient line")?;

        let Some(Value::Array(pattern)) = json.get("pattern") else {
            return Err(anyhow!("expected array at path 'pattern'"));
        };
        
        pattern.iter()
            .map(|line| parse_line(line, &key))
            .collect::<Result<Vec<Vec<Ingredient>>>>()
            .with_context(|| "while parsing pattern line")
            .map(Recipe::Shaped)
    }

    fn parse_shapeless(json: &Value) -> Result<Recipe> {
        let Some(Value::Array(array)) = json.get("ingredients") else {
            return Err(anyhow!("expected array at path 'ingredients'"));
        };

        array.iter()
            .map(parse_ingredient)
            .collect::<Result<Vec<Ingredient>>>()
            .with_context(|| "while parsing shapeless recipe")
            .map(Recipe::Shapeless)
    }

    let recipe = match category.as_ref() {
        "minecraft:crafting_shaped" => parse_shaped(&json).with_context(|| "while parsing shaped recipe"),
        "minecraft:crafting_shapeless" => parse_shapeless(&json).with_context(|| "while parsing shapeless recipe"),
        _ => Err(anyhow!("invalid category {}", category)),
    }?;

    let Some(result) = json.get("result").and_then(|o| o.get("id")).and_then(Value::as_str) else {
        return Err(anyhow!("did not find valid result key"));
    };
    
    Ok((result.to_owned(), recipe))
}

/// Parses a line of ingredients using the provided key.
fn parse_line(line: &Value, key: &HashMap<&String, Ingredient>) -> Result<Vec<Ingredient>> {
    let Some(line) = line.as_str() else {
        return Err(anyhow!("could not parse line: expected string, found {:?}", line));
    };

    line.chars().map(|c| {
        if c == ' ' {
            Ok(vec![Material::default()])
        } else {
            key.get(&c.to_string()).ok_or_else(|| anyhow!("unknown item type {}", c)).cloned()
        }
    })
    .collect::<Result<Vec<_>, _>>()
}

/// Parses an ingredient from JSON.
/// This will fully expand all tags.
fn parse_ingredient(value: &Value) -> Result<Ingredient> {
    if let Value::Object(object) = value {
        if object.len() != 1 {
            return Err(anyhow!("invalid input length: {:?}", object));
        }

        let Some((key, Value::String(str))) = object.iter().next() else {
            return Err(anyhow!("invalid input first entry: expected (string => string)"));
        };

        match key.as_ref() {
            "item" => Ok(
                if let Some(material) = material_from_str(str) {
                    vec![material]
                } else {
                    vec![]
                }
            ),
            "tag" => parse_tag(str)
                .with_context(|| format!("while parsing ingredient {:?}", value)),
            t => Err(anyhow!("invalid ingredient type {}", t)),
        }
    } else if let Value::Array(array) = value {
        let mut results = Vec::new();

        for elem in array {
            let mut appended = parse_ingredient(elem)
                .with_context(|| "while parsing list of ingredients")?;

            results.append(&mut appended);
        }

        Ok(results)
    } else {
        Err(anyhow!("Invalid input type of ingredient: {:?}", value))
    }
}

/// Expands a tag into a list of ingredients.
/// This will fully read any relevant tag files each time.
fn parse_tag(name: &str) -> Result<Ingredient> {
    if !name.starts_with("minecraft:") {
        return Err(anyhow!("invalid name: {}", name));
    }

    let name = &name["minecraft:".len()..];

    let mut path = PathBuf::from_str(TAG_DIRECTORY).unwrap();
    path.push(format!("{name}.json"));

    let json = read_json(&path)
        .with_context(|| format!("while parsing tag '{name}'"))?;

    let Some(inputs) = json.get("values").and_then(Value::as_array) else {
        return Err(anyhow!("could not find JSON array at path 'values'"));
    };

    let Some(inputs) = inputs.into_iter().map(Value::as_str).collect::<Option<Vec<_>>>() else {
        return Err(anyhow!("non-string value in JSON array 'values'"));
    };

    let mut result = Vec::new();

    for string in inputs {
        if string.starts_with("#") {
            let mut parsed = parse_tag(&string["#".len()..])
                .with_context(|| format!("while parsing tag '{name}'"))?;
            result.append(&mut parsed);
        } else {
            if let Some(material) = material_from_str(string) {
                result.push(material);
            }
        }
    }

    Ok(result)
}

/// Converts a string to a material.
pub fn material_from_str(str: &str) -> Option<Material> {
    Some(match str {
        "minecraft:air" => Material::Air,
        "minecraft:oak_planks" => Material::Planks,
        "minecraft:cobblestone" => Material::Cobblestone,
        "minecraft:stone" => Material::Stone,
        "minecraft:glass" => Material::Glass,
        "minecraft:white_wool" => Material::Wool,
        "minecraft:stick" => Material::Stick,
        "minecraft:coal" => Material::Coal,
        "minecraft:diamond" => Material::Diamond,
        "minecraft:gold_ingot" => Material::GoldIngot,
        "minecraft:iron_ingot" => Material::IronIngot,
        "minecraft:redstone" => Material::Redstone,
        "minecraft:quartz" => Material::Quartz,
        "minecraft:oak_slab" => Material::Slab,
        "minecraft:oak_log" => Material::Log,
        "minecraft:iron_nugget" => Material::IronNugget,
        "minecraft:redstone_torch" => Material::RedstoneTorch,
        "minecraft:string" => Material::String,
        "minecraft:leather" => Material::Leather,
        _ => return None,
    })
}