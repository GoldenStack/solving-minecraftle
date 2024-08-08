use std::{collections::HashMap, fs::{read_dir, read_to_string, DirEntry}, iter::zip, vec};

use itertools::Itertools;
use serde_json::Value;

pub const RECIPE_PATH: &str = "./recipe";

pub const DEFAULT: &str = "minecraft:air";

// https://minecraftle.zachmanson.com/

fn main() {

    let valid_ingredients = vec![
        "minecraft:air", // allow air since it can technically be guessed
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

    let shaped_offsets = |width, height| match (width, height) {
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
    };

    let paths = read_dir(RECIPE_PATH).unwrap();

    let recipes = paths
        .map(read_recipe)
        .map(parse_recipe)
        .filter(Option::is_some)
        .map(Option::unwrap)
        .collect::<Vec<_>>();

    println!("{} relevant recipes (shaped or shapeless)", recipes.len());

    let recipes = recipes.into_iter().filter_map(|recipe| match recipe {
        (recipe, Recipe::Shaped(grid)) => {
            grid.into_iter().map(|i| filter_ingredients(i, &valid_ingredients)).collect::<Option<Vec<_>>>()
                .map(|result| (recipe, Recipe::Shaped(result)))
        },
        (recipe, Recipe::Shapeless(ingredients)) => {
            filter_ingredients(ingredients, &valid_ingredients)
                .map(|result| (recipe, Recipe::Shapeless(result)))
        },
    }).collect_vec();

    println!("{} recipes following the item filter", recipes.len());

    let guess_combinations = recipes.iter().map(|(_, r)| r)
        .map(iter_recipe_guess)
        .flatten().collect_vec();
    let answer_combinations = recipes.iter().map(|(_, r)| r)
        .filter(|r| matches!(r, Recipe::Shaped(_))) // Remove shapeless recipes as results
        .map(|r| iter_recipe_answer(r, shaped_offsets))
        .flatten().collect_vec();

    println!("{} total recipe guesses; {} total recipe answers", guess_combinations.len(), answer_combinations.len());

    fn determine(answers: &Vec<Craft>, guesses: &Vec<Craft>) -> usize{
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
                println!("Final guess {:?}", min.1.iter().map(|v| &v["minecraft:".len()..]).collect_vec());
                1
            } else {
                println!("1 remaining: {:?}", min.0[0].iter().map(|v| &v["minecraft:".len()..]).collect_vec());
                2
            }
        } else {
            1 + determine(&min.0, guesses)
        }
    }

    let count = determine(&answer_combinations, &guess_combinations);
    println!("Attempts: {:?}", count);
     
}

fn read_recipe(input: Result<DirEntry, std::io::Error>) -> Value {
    let filename = input.unwrap().file_name();
    let name = filename.to_str().unwrap();

    let recipe = read_to_string(format!("{RECIPE_PATH}/{name}")).unwrap();
    
    serde_json::from_str::<Value>(&recipe).unwrap()
}

fn parse_recipe(json: Value) -> Option<(String, Recipe)> {
    let recipe = match json.get("type").unwrap() {
        Value::String(s) if s == "minecraft:crafting_shaped" => {
            let key = json.get("key").unwrap()
                .as_object().unwrap()
                .iter().map(|(k, v)| (k, parse_ingredient(v))).collect::<HashMap<_, _>>();            

            let pattern = json.get("pattern").unwrap()
                .as_array().unwrap()
                .iter().map(|line| parse_line(line, &key))
                .collect::<Vec<_>>();
            
            Recipe::Shaped(pattern)
        },
        Value::String(s) if s == "minecraft:crafting_shapeless" => {
            let ingredients = json.get("ingredients").unwrap()
                .as_array().unwrap()
                .iter().map(parse_ingredient).collect::<Vec<_>>();

            Recipe::Shapeless(ingredients)
        },
        _ => return None,
    };

    let result = json.get("result").unwrap().get("id").unwrap().as_str().unwrap().to_owned();
    Some((result, recipe))
}

fn parse_line(line: &Value, key: &HashMap<&String, Ingredient>) -> Vec<Ingredient> {
    line.as_str().unwrap()
        .chars().map(|c| {
            if c == ' ' {
                vec![DEFAULT.to_owned()]
            } else {
                key.get(&c.to_string()).unwrap().clone()
            }
        })
        .collect::<Vec<_>>()
}

fn parse_ingredient(value: &Value) -> Ingredient {
    match value {
        Value::Object(object) => {
            if object.len() != 1 {
                panic!("invalid input length: {:?}", object);
            }

            match object.iter().next().unwrap() {
                (key, Value::String(str)) if key == "item" => vec![str.to_owned()],
                (key, Value::String(str)) if key == "tag" => expand_tag(str),
                i => panic!("invalid ingredient: {:?}", i),
            }
        },
        Value::Array(array) => array.iter().map(parse_ingredient).flatten().collect(),
        i => panic!("invalid input type: {:?}", i),
    }
}

fn expand_tag(name: &str) -> Ingredient {
    if !name.starts_with("minecraft:") {
        panic!("invalid name: {}", name);
    }

    let name = &name["minecraft:".len()..];

    let string = read_to_string(format!("./tags/item/{name}.json")).unwrap();
    let json = serde_json::from_str::<Value>(&string).unwrap();

    let mut values = Vec::new();
    for value in json.get("values").unwrap().as_array().unwrap() {
        let string = value.as_str().unwrap();

        if string.starts_with("#") {
            expand_tag(&string["#".len()..]).iter().for_each(|i| values.push(i.to_owned()));
        } else {
            values.push(string.to_owned());
        }
    }

    values
}

fn filter_ingredients(ingredients: Vec<Ingredient>, filter: &Vec<&str>) -> Option<Vec<Ingredient>> {
    let applied = ingredients.into_iter()
        .map(|items| items.into_iter().filter(|item| filter.contains(&&**item)).collect_vec()).collect_vec();

    Some(applied).filter(|v| !v.iter().any(Vec::is_empty))
}

fn iter_shapeless<'a>(ingredients: &'a Vec<Ingredient>) -> Vec<Craft<'a>> {

    // A list of every possible slot combination for the ingredients
    let slots = (0..9usize).permutations(ingredients.len());

    // A list of every possible combination of ingredients
    // This is useful because an ingredient may have many different valid items
    let ingredient_combinations = ingredients.iter().multi_cartesian_product();

    // Sort the list and deduplicate.
    let unique_combinations = ingredient_combinations.map(|mut i| {
        i.sort();
        i
    }).unique();

    // Generate a combination of every ingredient with every set of valid slots
    let combinations = Itertools::cartesian_product(unique_combinations, slots);

    // At this point, it should be possible to have already deduplicated them.
    // It's impossible to optimize recipes that have many different options as
    // inputs, but recipes with large numbers of the same input (e.g. 9) should
    // be easy to deduplicate.

    // Map them to a craft
    combinations.map(|(l, r)| {
        let mut craft: [&str; 9] = [DEFAULT; 9];
        for (l, r) in zip(l, r) {
            craft[r] = l;
        }
        craft
    }).unique().collect()
}

fn iter_shaped_guess<'a>(grid: &'a Vec<Vec<Ingredient>>) -> Vec<Craft<'a>> {
    let mut crafts = Vec::new();

    let width = grid.iter().map(Vec::len).min().unwrap_or(0);
    let height = grid.len();

    for ingredients in grid.iter().flatten().multi_cartesian_product() {
        for (ox, oy) in Itertools::cartesian_product(0..=3-width, 0..=3-height) {
            let mut craft: [&str; 9] = [DEFAULT; 9];

            for (x, y) in Itertools::cartesian_product(0..width, 0..height) {
                craft[x + ox + (y + oy) * 3] = ingredients.get(x + y * width).unwrap();
            }

            crafts.push(craft);
        }
    }

    crafts
}

fn iter_shaped_answer<'a, F: Fn(usize, usize) -> (usize, usize)>(grid: &'a Vec<Vec<Ingredient>>, map: F) -> Vec<Craft<'a>> {

    let mut crafts = Vec::new();

    let width = grid.iter().map(Vec::len).min().unwrap_or(0);
    let height = grid.len();

    let (ox, oy) = map(width, height);

    for ingredients in grid.iter().flatten().multi_cartesian_product() {
        let mut craft: [&str; 9] = [DEFAULT; 9];

        for (x, y) in Itertools::cartesian_product(0..width, 0..height) {
            craft[x + ox + (y + oy) * 3] = ingredients.get(x + y * width).unwrap();
        }

        crafts.push(craft);
    }

    crafts
}

fn iter_recipe_guess<'a>(recipe: &'a Recipe) -> Vec<Craft<'a>> {
    match recipe {
        Recipe::Shaped(grid) => iter_shaped_guess(grid),
        Recipe::Shapeless(ingredients) => iter_shapeless(ingredients)
    }
}

fn iter_recipe_answer<'a, F: Fn(usize, usize) -> (usize, usize)>(recipe: &'a Recipe, map: F) -> Vec<Craft<'a>> {
    match recipe {
        Recipe::Shaped(grid) => iter_shaped_answer(grid, map),
        Recipe::Shapeless(ingredients) => iter_shapeless(ingredients)
    }
}

fn calculate_hint(answer: &Craft, guess: &Craft) -> Hint {
    if answer == guess {
        return [Color::Green; 9];
    }

    let mut hint = [Color::Gray; 9];

    let mut used = [false; 9];

    // Greens - exact correct spot
    for (index, (l, r)) in zip(answer, guess).enumerate() {
        if l == r && l != &DEFAULT {
            hint[index] = Color::Green;
            used[index] = true;
        }
    }

    // Yellows
    'outer:
    for letter in answer {
        for (index, guess_letter) in guess.iter().enumerate() {
            if !used[index] && letter == guess_letter && guess_letter != &DEFAULT {
                hint[index] = Color::Yellow;
                used[index] = true;
                continue 'outer;
            }
        }
    }

    hint
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
