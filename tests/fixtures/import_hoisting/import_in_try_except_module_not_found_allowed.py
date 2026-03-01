def safe_import():
    try:
        import optional_module
        tracer = optional_module.tracer()
    except ModuleNotFoundError:
        tracer = None
