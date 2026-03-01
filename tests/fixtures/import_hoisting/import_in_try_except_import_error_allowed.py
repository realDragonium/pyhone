def safe_import():
    try:
        import optional_module
    except ImportError:
        pass
