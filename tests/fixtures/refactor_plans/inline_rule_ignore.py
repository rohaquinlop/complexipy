def narrowed(a, b, c, d):  # complexipy: ignore[C007]
    if a:
        if b:
            if c and d:
                return 1
    return 0


def noqa_narrowed(a, b, c, d):  # noqa: complexipy[C007]
    if a:
        if b:
            if c and d:
                return 2
    return 0
