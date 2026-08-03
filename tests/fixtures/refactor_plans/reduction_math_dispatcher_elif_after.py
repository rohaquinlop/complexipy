def sample(kind):
    handlers = {"a": lambda: 1, "b": lambda: 2, "c": lambda: 3, "d": lambda: 4}
    handler = handlers.get(kind)
    return handler() if handler else 0
