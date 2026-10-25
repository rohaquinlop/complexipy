def f(xs, a, b):
    total = 0
    for x in xs:
        if a:
            total += 1
            if b:
                total += x
        total += 100
    return total
