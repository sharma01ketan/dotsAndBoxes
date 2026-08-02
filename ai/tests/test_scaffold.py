from dab_ai import __version__
from dab_ai.cli import main


def test_version() -> None:
    assert __version__ == "0.1.0"


def test_cli_main(capsys) -> None:
    main()
    captured = capsys.readouterr()
    assert "dab-ai" in captured.out
    assert "scaffold ready" in captured.out
