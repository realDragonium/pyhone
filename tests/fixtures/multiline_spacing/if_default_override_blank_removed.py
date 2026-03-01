def process(dto):
    created_by_mask = None

    if dto.created_by:
        created_by_mask = get_mask(dto.created_by)

    return created_by_mask
