def process(items):
    total = 0
    for x in items:
        if x < 0 or x > 100:
            total += x
            if total > 100:
                return total
    return total
