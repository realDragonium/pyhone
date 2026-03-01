def collect(items):
    results = build(
        items,
        extra=True,
    )

    for item in results:
        process(item)

    return results
