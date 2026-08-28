def process(items, scale):
    total = 0
    for x in items:
        if x > 0:
            total += calculate(
                x, scale,
            )
            if total > 100:
                return total
    return total


def calculate(x, scale):
    return x * scale
