if True:
    LIMIT = 10


def f(x, y):
    if x > 0:
        return 1
    else:
        if y < 0 and y > -LIMIT or y == 5:
            return 2
    return 0
