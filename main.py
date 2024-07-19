import os
import json
import itertools
import heapq

# Get files
files = os.listdir("./recipe")

# Read JSON from each file
files = list(map(lambda f: open(f"./recipe/{f}", "r").read(), files))
recipes = list(map(json.loads, files))
print(f"{len(recipes)} recipes initially")

# Filter crafting table recipes (shaped or shapeless)
recipes = list(filter(lambda r: "crafting_shaped" in r["type"] or "crafting_shapeless" in r["type"], recipes))
print(f"{len(recipes)} shaped or shapeless")

# Filter out the list of required items from each recipe.
# Some recipes allow multiple inputs, so we adjust for those too.
def extract_tuple(map: dict) -> tuple[str, str]:
    if len(map) != 1:
        raise ValueError()
    for (k, v) in map.items():
        return (k, v)

def get_items(data) -> list[list[tuple[str, str]]]:
    ingredients = []
    if "crafting_shaped" in data["type"]:
        for value in data["key"].values():
            if type(value) != list:
                value = [value]
            ingredients.append(list(map(extract_tuple, value)))
    elif "crafting_shapeless" in data["type"]:
        for value in data["ingredients"]:
            if type(value) != list:
                value = [value]
            ingredients.append(list(map(extract_tuple, value)))

    return ingredients

recipes = list(map(get_items, recipes))

# Replace every tag with its constituent items
def lookup_tag(tag: str) -> list[str]:
    items = []
    name = tag.split(":")[1]
    for line in json.loads(open(f"./tags/item/{name}.json", "r").read())["values"]:
        if line.startswith("#"):
            items += lookup_tag(line[1::])
        else:
            items.append(line)
    return items

def expand_tags(ingredient: list[tuple[str, str]]) -> list[str]:
    new_items: list[str] = []
    for item in ingredient:
        if item[0] == "item":
            new_items.append(item[1])
        if item[0] == "tag":
            new_items += lookup_tag(item[1])
    return new_items

recipes = list(map(lambda r: list(map(expand_tags, r)), recipes))

# Filter out only recipes with our ingredients
ingredients = {
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
}

# Remove all except relevant elements
recipes = list(map(lambda r: list(map(lambda i: list(filter(lambda id: id in ingredients, i)), r)), recipes))
# Now, remove any with empty items
recipes = list(filter(lambda r: all(r), recipes))

print(f"{len(recipes)} recipes have exclusively our {len(ingredients)} ingredients")

# Calculate the cartesian product of each element, and add them together
sets = list(itertools.chain.from_iterable(list(map(lambda recipe: list(map(frozenset, itertools.product(*recipe))), recipes))))

print("Recipe makeup:")
for i in ingredients:
    count = len(list(filter(lambda set: i in set, sets)))
    print(f"    {i} has {count} recipe(s)")

# Find only unique elements
sets = set(sets)

# Remove sets that are subsets of other sets
# This is O(n^2) time complexity, which is quite bad, but this is fine with such a small n
sets = list(filter(lambda set: not any(set.issubset(set2) and set != set2 for set2 in sets), sets))

# Apply set cover algorithm (solution from https://stackoverflow.com/questions/21973126/set-cover-or-hitting-set-numpy-least-element-combinations-to-make-up-full-set)
def greedy_set_cover(subsets, parent_set):
    parent_set = set(parent_set)
    max = len(parent_set)
    # create the initial heap. Note 'subsets' can be unsorted,
    # so this is independent of whether remove_redunant_subsets is used.
    heap = []
    for s in subsets:
        # Python's heapq lets you pop the *smallest* value, so we
        # want to use max-len(s) as a score, not len(s).
        # len(heap) is just proving a unique number to each subset,
        # used to tiebreak equal scores.
        heapq.heappush(heap, [max-len(s), len(heap), s])
    results = []
    result_set = set()
    while result_set < parent_set:
        best = []
        unused = []
        while heap:
            score, count, s = heapq.heappop(heap)
            if not best:
                best = [max-len(s - result_set), count, s]
                continue
            if score >= best[0]:
                # because subset scores only get worse as the resultset
                # gets bigger, we know that the rest of the heap cannot beat
                # the best score. So push the subset back on the heap, and
                # stop this iteration.
                heapq.heappush(heap, [score, count, s])
                break
            score = max-len(s - result_set)
            if score >= best[0]:
                unused.append([score, count, s])
            else:
                unused.append(best)
                best = [score, count, s]
        add_set = best[2]
        results.append(add_set)
        result_set.update(add_set)
        # subsets that were not the best get put back on the heap for next time.
        while unused:
            heapq.heappush(heap, unused.pop())
    return results

solution = greedy_set_cover(set(sets), ingredients)

for item in solution:
    print(item)
print(f"took {len(solution)} items!")

# Piston
# Comparator
# Campfire
# Daylight Sensor
# Powered Rail
# Chain
# Bow
# Item Frame
# Jukebox
# Bed