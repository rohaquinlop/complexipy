def f(xs, a, b):
    out = []
    for x in xs:
        if a:
            out.append(0)
            if b:
                for y in xs:
                    out.append(y)
            out.append(9)
    return out
