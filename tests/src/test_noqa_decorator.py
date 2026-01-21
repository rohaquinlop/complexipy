@staticmethod
def ignored_decorated_function(a, b):  # noqa: complexipy
    if a and b:
        return a + b
    elif a or b:
        return a or b
    else:
        return 0


def simple_function(a, b):
    return range(a, b)
