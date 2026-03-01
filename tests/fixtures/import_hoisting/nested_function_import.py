def outer():
    def inner():
        import json
        return json.dumps({})
    return inner()
