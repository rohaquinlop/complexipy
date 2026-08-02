import json


def read_config(path):
    try:
        with open(path) as f:
            try:
                return json.load(f)
            except json.JSONDecodeError:
                return {}
    except FileNotFoundError:
        return {}
