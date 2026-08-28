def process(items):
    total = 0
    for x in items:
        if x > 0:
            total += x
            if total > 100:
                return total
    return total
