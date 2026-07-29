def sample(a, b, c, d):
    if a and b:
        if c:
            for value in d:
                if value.ready:
                    return value
    return None
