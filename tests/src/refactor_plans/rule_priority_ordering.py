def sample(data, config, user):
    if data:
        if data.is_valid():
            if config.enabled:
                if user.has_permission:
                    return process(data)
    return None
