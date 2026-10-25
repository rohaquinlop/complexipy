def f(xs, a, b, c):
    out = []
    for x in xs:
        if not a or(b):
            out.append(0)
            if c:
                out.append(x)
    return out
