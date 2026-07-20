def multi_byte_before_marker(a):  # noqa: complexipy — multi-byte em-dash before marker
    if a:
        return a
    elif a > 10:
        return a * 2
    else:
        return 0


def emoji_comment(a, b):  # complexipy: ignore 🔥 multi-byte emoji after marker
    if a and b:
        return a + b
    elif a or b:
        return a | b
    else:
        return 0


def accent_before(b):  # noqa: complexipy café multi-byte accent after marker
    if b:
        return b
    elif b < 5:
        return b * 3
    else:
        return 0


def mixed_comment_marker(
    a, b, c
):  # complexipy: ignore — 🚀 mixed em-dash & emoji
    if a and b:
        return a + b
    elif b and c:
        return b * c
    elif a or c:
        return a | c
    else:
        return 0


def uppercase_variants(x):  # NOQA: COMPLEXIPY — uppercase variant
    if x:
        return x
    elif x > 10:
        return x * 2
    else:
        return 0


def mixed_case_ignore(y):  # Complexipy: Ignore — mixed case
    if y:
        return y * 2
    elif y < 0:
        return -y
    else:
        return 0


def not_ignored_normal(a, b):
    if a and b:
        return a + b
    elif a or b:
        return a | b
    else:
        return 0
