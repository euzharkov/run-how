import nox

@nox.session
def lint(session):
    """Run linters."""
    session.run("ruff", "check", ".")

@nox.session(python=["3.11", "3.12"])
def tests(session):
    session.run("pytest")
