def process(value):
    created_at = value.created_at

    if created_at is None:
        created_at = now()

    return created_at
