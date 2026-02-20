# Test fixtures for comprehension complexity scoring.
#
# Scoring rules implemented:
#   - Each comprehension (list/set/dict/generator): +1 + nesting_level
#   - Each additional `for` clause beyond the first: +1
#   - Each `if` filter: +1  (plus any boolean operators inside the condition)
#   - Nested comprehensions get the depth penalty applied recursively


def simple_listcomp(items):
    # [x for x in items] at nesting_level=0 → +1
    return [x for x in items]


def listcomp_with_if(items):
    # +1 (comprehension) +1 (if clause) = 2
    return [x for x in items if x > 0]


def listcomp_two_for(matrix):
    # +1 (comprehension) +1 (second for) = 2
    return [x for row in matrix for x in row]


def listcomp_two_for_with_if(matrix):
    # +1 (comprehension) +1 (second for) +1 (if) = 3
    return [x for row in matrix for x in row if x > 0]


def nested_listcomp(matrix):
    # outer: +1 (at nesting_level=0 inside function)
    # inner elt: count_comprehension_complexity at nesting_level=1 → +1+1=2
    # total: 1 + 2 = 3
    return [[x for x in row] for row in matrix]


def genexp_in_call(items):
    # generator expression: +1 + nesting_level(0) = 1
    return sum(x * 2 for x in items)


def dictcomp_simple(keys):
    # +1 (comprehension) = 1
    return {k: k * 2 for k in keys}


def dictcomp_with_if(keys):
    # +1 (comprehension) +1 (if) = 2
    return {k: k * 2 for k in keys if k > 0}


def setcomp_simple(items):
    # +1 = 1
    return {x for x in items}
