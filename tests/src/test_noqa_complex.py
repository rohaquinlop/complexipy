def ignored_complex_function(a, b):  # noqa: complexipy (legacy API wrapper; safe to skip)
    if a and b:
        return a + b
    elif a or b:
        return a or b
    else:
        return 0


def not_ignored_function(a, b):
    return range(a, b)
