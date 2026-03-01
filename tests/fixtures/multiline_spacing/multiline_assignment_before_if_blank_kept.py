def process(dto):
    created_at = compute(
        dto.value,
        default=None,
    )

    if created_at is None:
        created_at = now()

    return created_at
