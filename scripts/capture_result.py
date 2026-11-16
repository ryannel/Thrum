"""An incomplete engine interval cannot judge the captured build."""


class CaptureUnjudged(ValueError):
    """Propagate a declared incomplete capture, without accepting its output."""

    exit_code = 75
