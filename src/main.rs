pub mod parse;
pub mod permutations;

use std::{collections::HashMap, iter::zip};

use anyhow::{Context, Result};
use itertools::Itertools;
use permutations::{permutations_answer, permutations_guess};

use parse::*;

/// The directory containing all relevant recipes.
/// This can be directly grabbed from Minecraft's data folder.
pub const RECIPE_DIRECTORY: &str = "./recipe/";

/// The directory containing all relevant tags.
/// This can be directly grabbed from Minecraft's data folder.
pub const TAG_DIRECTORY: &str = "./tags/item/";

/// The default material name for "empty" recipe slots.
pub const DEFAULT_ITEM: &str = "minecraft:air";

pub const VALID_INGREDIENTS: [&str; 18+1] = [
    "minecraft:air", // Allow air since it can technically be guessed
    "minecraft:oak_planks",
    "minecraft:cobblestone",
    "minecraft:stone",
    "minecraft:glass",
    "minecraft:white_wool",
    "minecraft:stick",
    "minecraft:coal",
    "minecraft:diamond",
    "minecraft:gold_ingot",
    "minecraft:iron_ingot",
    "minecraft:redstone",
    "minecraft:quartz",
    "minecraft:oak_slab",
    "minecraft:oak_log",
    "minecraft:iron_nugget",
    "minecraft:redstone_torch",
    "minecraft:string",
    "minecraft:leather"
];

fn main() -> Result<()> {

    let files = list_dir(RECIPE_DIRECTORY)
        .with_context(|| "while listing files in recipe directory")?;

    let json = files.into_iter()
        .map(|path| match read_json(&path) {
            Ok(json) => Ok((path, json)),
            Err(err) => Err(err.context(format!("while parsing path {path:?}")))
        })
        .collect::<Result<Vec<_>>>()
        .with_context(|| "while parsing recipe JSON")?;
    
    println!("{} total recipes", json.len());

    let recipes = json.into_iter()
        .filter(|(_, json)| filter_recipe(json))
        .map(|(path, json)| parse_recipe(json)
            .with_context(|| format!("while parsing path {path:?}"))
        )
        .collect::<Result<Vec<_>, _>>()
        .with_context(|| "while parsing recipes from JSON")?;

    println!("{} relevant recipes (shaped or shapeless)", recipes.len());

    let recipes = recipes.into_iter()
        .filter_map(|(result, recipe)|
            filter_ingredients(recipe, &VALID_INGREDIENTS)
            .map(|recipe| (result, recipe))
        )
        .collect_vec();

    println!("{} filtered recipes (containing valid items)", recipes.len());

    let guesses = recipes.iter()
        .map(|r| permutations_guess(&r.1))
        .flatten()
        .collect_vec();

    let answers = recipes.iter()
        .map(|r| permutations_answer(&r.1, get_shaped_offset))
        .flatten()
        .collect_vec();

    println!("{} total recipe guesses; {} total recipe answers", guesses.len(), answers.len());

    let count: usize = worst_case(&answers, &guesses);
    println!("Worst case attempts: {:?}", count);

    Ok(())
}

#[derive(Debug)]
pub enum Recipe {
    Shaped(Vec<Vec<Ingredient>>),
    Shapeless(Vec<Ingredient>),
}

pub type Ingredient = Vec<String>;

pub type Craft<'a> = [&'a str; 9];

pub type Hint = [Color; 9];

// I tried naming these something other than the color but they were too verbose
#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
pub enum Color {
    Gray, Yellow, Green
}

fn get_shaped_offset(size: (usize, usize)) -> (usize, usize) {
    match size {
        (1, 1) => (1, 1),
        (1, 2) => (1, 0),
        (1, 3) => (1, 0),
        (2, 1) => (0, 0),
        (2, 2) => (0, 0),
        (2, 3) => (0, 0),
        (3, 1) => (0, 0),
        (3, 2) => (0, 0),
        (3, 3) => (0, 0),
        _ => unreachable!()
    }
}

fn worst_case(answers: &Vec<Craft>, guesses: &Vec<Craft>) -> usize {
    let min = guesses.iter()
    .map(|guess| assemble_pools(&guess, &answers))
    .map(|pools| pools.values().max_by_key(|v| v.len()).unwrap().clone())
    .zip(guesses)
    .min_set_by_key(|(a, _)| a.len());

    let min = min.get(0).unwrap();

    let mapped = min.1.iter().map(|v| &v["minecraft:".len()..]).collect_vec();

    println!("From {:?} to {:?} possible solutions by {:?}", answers.len(), min.0.len(), mapped);

    if min.0.len() == 1 {
        if &min.0[0] == min.1 {
            1
        } else {
            // Simulate another guess
            1 + worst_case(&min.0, &min.0)
        }
    } else {
        1 + worst_case(&min.0, guesses)
    }
}

fn assemble_pools<'a>(guess: &Craft, answers: &'a Vec<Craft>) -> HashMap<Hint, Vec<Craft<'a>>> {
    let mut map: HashMap<_, Vec<_>> = HashMap::new();

    for answer in answers.clone() {
        let overlap = calculate_hint(&answer, guess);
        
        if let Some(vec) = map.get_mut(&overlap) {
            vec.push(answer);
        } else {
            map.insert(overlap, vec![answer]);
        }
    }

    map
}

fn calculate_hint(answer: &Craft, guess: &Craft) -> Hint {
    if answer == guess {
        return [Color::Green; 9];
    }

    let mut hint = [Color::Gray; 9];

    let mut used = [false; 9];

    // Greens - exact correct spot
    for (index, (l, r)) in zip(answer, guess).enumerate() {
        if l == r && l != &DEFAULT_ITEM {
            hint[index] = Color::Green;
            used[index] = true;
        }
    }

    // Yellows
    'outer:
    for letter in answer {
        for (index, guess_letter) in guess.iter().enumerate() {
            if !used[index] && letter == guess_letter && guess_letter != &DEFAULT_ITEM {
                hint[index] = Color::Yellow;
                used[index] = true;
                continue 'outer;
            }
        }
    }

    hint
}
