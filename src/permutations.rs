use itertools::Itertools;
use std::iter::zip;

use crate::{Craft, Ingredient, Recipe, DEFAULT_ITEM};

pub fn permutations_guess<'a>(recipe: &'a Recipe) -> Vec<Craft<'a>> {
    match recipe {
        Recipe::Shaped(grid) => permutations_shaped(grid),
        Recipe::Shapeless(ingredients) => permutations_shapeless(ingredients),
    }
} 

pub fn permutations_answer<'a, F: Fn((usize, usize)) -> (usize, usize)>(recipe: &'a Recipe, location: F) -> Vec<Craft<'a>> {
    match recipe {
        Recipe::Shaped(grid) => permutations_shaped_for(grid, location(grid_size(grid))),
        Recipe::Shapeless(_) => Vec::new(),
    }
}

fn permutations_shapeless<'a>(ingredients: &'a Vec<Ingredient>) -> Vec<Craft<'a>> {

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
        let mut craft: [&str; 9] = [DEFAULT_ITEM; 9];
        for (l, r) in zip(l, r) {
            craft[r] = l;
        }
        craft
    }).unique().collect()
}

fn permutations_shaped<'a>(grid: &'a Vec<Vec<Ingredient>>) -> Vec<Craft<'a>> {
    let width = grid.iter().map(Vec::len).min().unwrap_or(0);
    let height = grid.len();

    Itertools::cartesian_product(0..=3-width, 0..=3-height)
        .map(|offset| permutations_shaped_for(grid, offset))
        .flatten().collect_vec()
}

fn permutations_shaped_for<'a>(grid: &'a Vec<Vec<Ingredient>>, (ox, oy): (usize, usize)) -> Vec<Craft<'a>> {
    let mut crafts = Vec::new();

    let (width, height) = grid_size(grid);

    for ingredients in grid.iter().flatten().multi_cartesian_product() {
        let mut craft: [&str; 9] = [DEFAULT_ITEM; 9];

        for (x, y) in Itertools::cartesian_product(0..width, 0..height) {
            craft[x + ox + (y + oy) * 3] = ingredients.get(x + y * width).unwrap();
        }

        crafts.push(craft);
    }

    crafts.into_iter().unique().collect_vec()
}

fn grid_size(grid: &Vec<Vec<Ingredient>>) -> (usize, usize) {
    (grid.iter().map(Vec::len).min().unwrap_or(0), grid.len())
}