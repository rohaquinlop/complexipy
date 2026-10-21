def f(xs, a, b):
    total = 0
    i = 0
    while i < len(xs):
        if a:
            total += 1
            if b:
                total += xs[i]
        i += 1
    return total
