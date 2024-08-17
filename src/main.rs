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

    // Normally we would have to filter out recipes here that have ingredients
    // with 0 materials, but this is not an issue as the iterated Cartesian
    // product of them will result in a 0-length list anyway.
    
    println!("{} relevant recipes (shaped or shapeless)", recipes.len());

    let recipes = recipes.into_iter()
        .filter(|(_, recipe)| matches!(recipe, Recipe::Shaped(_)))
        .collect_vec();

    println!("{} filtered and relevant recipes (removed shapeless)", recipes.len());

    let guesses = recipes.iter()
        .map(|r| permutations_guess(&r.1))
        .flatten()
        .collect_vec();

    let answers = recipes.iter()
        .map(|r| permutations_answer(&r.1, get_shaped_offset))
        .flatten()
        .collect_vec();

    println!("{} total recipe guesses; {} total recipe answers", guesses.len(), answers.len());

    // 255C2 calculator
    let min = guesses.iter().combinations(2)
        .map(|vec| {
            let mut hint_map = HashMap::new();

            for answer in &answers {
                let hints = vec.iter().map(|guess| calculate_hint(answer, guess)).collect_vec();

                *hint_map.entry(hints).or_insert(0) += 1;
            }
            
            ((vec), *hint_map.values().max().unwrap())
        }).min_set_by_key(|(_, v)| *v);

    for (vec, count) in min {
        println!("{} from [{}]", count, vec.iter().map(|v| fmt(v)).join("], ["));
    }


    // Greedy algorithm stuff
    // println!("Guesses: {}", greedy_algorithm_against(&answers, &guesses, guess_from_user));

    // let data = answers.iter()
    //     .map(|answer| (answer, modified_greedy(&answers, &guesses, &[
    //         [
    //             Material::Planks, Material::Planks, Material::Planks,
    //             Material::Cobblestone, Material::IronIngot, Material::Cobblestone,
    //             Material::Cobblestone, Material::Redstone, Material::Cobblestone,
    //         ],
    //         [
    //             Material::GoldIngot, Material::GoldIngot, Material::GoldIngot,
    //             Material::Air, Material::Stick, Material::Air,
    //             Material::Air, Material::Stick, Material::Air,
    //         ],
    //         [
    //             Material::Air, Material::RedstoneTorch, Material::Air,
    //             Material::RedstoneTorch, Material::Quartz, Material::RedstoneTorch,
    //             Material::Stone, Material::Stone, Material::Stone,
    //         ],
    //     ], |guess| {
    //         // println!("{}", fmt(guess));
    //         calculate_hint(&answer, guess)
    //     }))).collect_vec();

    // let raw = data.iter().map(|(_, v)| *v).collect_vec();

    // let average = raw.iter().sum::<usize>() as f64 / (answers.len() as f64);
    // let min = raw.iter().min().unwrap();
    // let max = raw.iter().max().unwrap();

    // println!("average: {}, min: {}, max: {}", average, min, max);

    Ok(())
}

#[derive(Debug, PartialEq, Eq, Clone, Copy, Hash, PartialOrd, Ord)]
pub enum Material {
    Air,
    Planks,
    Cobblestone,
    Stone,
    Glass,
    Wool,
    Stick,
    Coal,
    Diamond,
    GoldIngot,
    IronIngot,
    Redstone,
    Quartz,
    Slab,
    Log,
    IronNugget,
    RedstoneTorch,
    String,
    Leather,
}

impl Default for Material {
    fn default() -> Self {
        Material::Air
    }
}

#[derive(Debug)]
pub enum Recipe {
    Shaped(Vec<Vec<Ingredient>>),
    Shapeless(Vec<Ingredient>),
}

pub type Ingredient = Vec<Material>;

pub type Craft<'a> = [Material; 9];

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

fn guess_from_user(guess: &Craft) -> Hint {
    println!("Guess is {}", fmt(guess));

    let mut input_text = String::new();
    std::io::stdin()
        .read_line(&mut input_text)
        .expect("failed to read from stdin");

    let mut colors = [Color::Gray; 9];
    
    for (index, color) in input_text.chars().enumerate() {
        if index >= 9 { break; }

        colors[index] = match color {
            'G' => Color::Green,
            'Y' => Color::Yellow,
            _ => Color::Gray,
        }
    }

    colors
}


fn fmt(guess: &Craft) -> String {
    guess.iter().map(|v| format!("{:?}", v)).collect_vec().join(" ")
}

/// Simulates the greedy algorithm against the provided answer.
fn greedy_algorithm_against_answer(answers: &Vec<Craft>, guesses: &Vec<Craft>, answer: &Craft) -> usize {
    greedy_algorithm_against(&answers, &guesses, |guess| {
        // println!("{}", fmt(guess));
        calculate_hint(&answer, guess)
    })
}

/// Calculates the guess that will result in the next guess specifically gaining
/// the most amount of information.
fn most_information<'a>(answers: &'a Vec<Craft>, guesses: &'a Vec<Craft>) -> Vec<(Vec<usize>, &'a Craft<'a>)> {
    let mut best_guesses = guesses.iter()
        .map(|guess| (assemble_pools(&guess, &answers).values().map(|v| v.len()).sorted().rev().collect_vec(), guess))
        .min_set_by_key(|(values, _)| values[0]);

    for index in 1..best_guesses.iter().map(|(_, r)| r.len()).max().unwrap_or(0) {
        best_guesses = best_guesses.into_iter().min_set_by_key(|(values, _)| values.get(index).cloned().unwrap_or(0));
    }

    best_guesses
}

/// Simulates a greedy algorithm against the provided guess function.
fn modified_greedy<F: Fn(&Craft) -> Hint>(answers: &Vec<Craft>, guesses: &Vec<Craft>, hardcoded: &[Craft], try_guess: F) -> usize {
    let best_guesses = most_information(answers, guesses);

    if best_guesses.iter().any(|v| v.0.len() == 1) {

        let best_guesses = best_guesses.iter().unique_by(|v| &v.0).collect_vec();

        let mut count = 0;
        for guess in &best_guesses {
            let result = try_guess(guess.1);
            count += 1;

            // println!("Exiting after {count:?} guesses from 1-large sets");
            if result == [Color::Green; 9] {
                return count;
            }
        }
    }

    let mut best_guess = best_guesses.get(0).unwrap().1;

    if hardcoded.len() > 0 {
        best_guess = &hardcoded[0];
    }

    let result = try_guess(best_guess);
    // println!("Guessed {:?}; result was {:?}", lowest_pair.1.iter().map(|v| &v["minecraft:".len()..]).collect_vec(), result);

    if result == [Color::Green; 9] {
        // println!("Exiting because we had all correct");
        return 1; // Took one guess
    }

    let pools = assemble_pools(best_guess, &answers);
    let new_answers = pools.get(&result).unwrap();

    if new_answers.len() == 1 {
        // println!("Fast exiting with one guess left: {:?}", new_answers.get(0).unwrap().iter().map(|v| &v["minecraft:".len()..]).collect_vec());
        println!("{}", fmt(new_answers.get(0).unwrap()));
        // 1 for initial guess + 1 for now
        return 1 + 1;
    }

    // println!("Simulating guess deeper...");
    let new_hardcoded = if hardcoded.len() == 0 { hardcoded } else { &hardcoded[1..] };
    return 1 + modified_greedy(new_answers, guesses, new_hardcoded, try_guess);
}

/// Simulates a greedy algorithm against the provided guess function.
fn greedy_algorithm_against<F: Fn(&Craft) -> Hint>(answers: &Vec<Craft>, guesses: &Vec<Craft>, try_guess: F) -> usize {
    let best_guesses = most_information(answers, guesses);

    if best_guesses.iter().any(|v| v.0.len() == 1) {

        let best_guesses = best_guesses.iter().unique_by(|v| &v.0).collect_vec();

        let mut count = 0;
        for guess in &best_guesses {
            let result = try_guess(guess.1);
            count += 1;

            // println!("Exiting after {count:?} guesses from 1-large sets");
            if result == [Color::Green; 9] {
                return count;
            }
        }
    }

    let lowest_pair = best_guesses.get(0).unwrap();

    let result = try_guess(lowest_pair.1);
    // println!("Guessed {:?}; result was {:?}", lowest_pair.1.iter().map(|v| &v["minecraft:".len()..]).collect_vec(), result);

    if result == [Color::Green; 9] {
        // println!("Exiting because we had all correct");
        return 1; // Took one guess
    }

    let pools = assemble_pools(lowest_pair.1, &answers);
    let new_answers = pools.get(&result).unwrap();

    if new_answers.len() == 1 {
        // println!("Fast exiting with one guess left: {:?}", new_answers.get(0).unwrap().iter().map(|v| &v["minecraft:".len()..]).collect_vec());
        println!("{}", fmt(new_answers.get(0).unwrap()));
        // 1 for initial guess + 1 for now
        return 1 + 1;
    }

    // println!("Simulating guess deeper...");
    return 1 + greedy_algorithm_against(new_answers, guesses, try_guess);
}

/// Simulates a greedy algorithm against an adversarial game.
/// This is pretty much the simplest case imaginable.
fn greedy_adversarial(answers: &Vec<Craft>, guesses: &Vec<Craft>) -> usize {
    let min = guesses.iter()
    .map(|guess| assemble_pools(&guess, &answers))
    .map(|pools| pools.values().max_by_key(|v| v.len()).unwrap().clone())
    .zip(guesses)
    .min_set_by_key(|(a, _)| a.len());

    let min = min.get(0).unwrap();

    println!("From {:?} to {:?} possible solutions by {:?}", answers.len(), min.0.len(), fmt(min.1));

    if min.0.len() == 1 {
        if &min.0[0] == min.1 {
            1
        } else {
            // Simulate another guess
            1 + greedy_adversarial(&min.0, &min.0)
        }
    } else {
        1 + greedy_adversarial(&min.0, guesses)
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
        if l == r && *l != Material::default() {
            hint[index] = Color::Green;
            used[index] = true;
        }
    }

    // Yellows
    'outer:
    for letter in answer {
        for (index, guess_letter) in guess.iter().enumerate() {
            if !used[index] && letter == guess_letter && *guess_letter != Material::default() {
                hint[index] = Color::Yellow;
                used[index] = true;
                continue 'outer;
            }
        }
    }

    hint
}
