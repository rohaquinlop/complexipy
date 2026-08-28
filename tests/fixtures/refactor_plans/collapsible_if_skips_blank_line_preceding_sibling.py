def collect(paths: list[str], root_dir: str, prefix: str) -> list[str]:
    modules: list[str] = []
    for path in paths:
        if root_dir in path:

            mod_path: str = path.removeprefix(prefix).replace("/", ".")

            if mod_path not in modules:
                modules.append(mod_path)
    return modules
