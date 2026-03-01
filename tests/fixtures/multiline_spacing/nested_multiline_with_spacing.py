def process(items):
    result = {}
    for i, item in enumerate(items):
        key = item

        value = build(
            item,
            index=i,
            extra=None,
        )

        result[i] = value

    return result
