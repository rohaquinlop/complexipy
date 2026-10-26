try:
    from _speedups import fast_path
except ImportError:  # 1 (module)

    def fast_path(values):
        for value in values:  # 1 (nested = 0)
            if value:  # 2 (nested = 1)
                return value
        return None


class Outer:
    class Inner:
        def method(self, flag):
            if flag:  # 1 (nested = 0)
                return 1
            return 0
