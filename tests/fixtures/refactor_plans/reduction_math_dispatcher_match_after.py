def sample(kind, data):
    handlers = {"a": _handle_a, "b": _handle_b, "c": _handle_c, "d": _handle_d}
    handler = handlers.get(kind)
    return handler(data) if handler else 0
