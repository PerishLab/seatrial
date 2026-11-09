from daily.common import until
from host import emit
from wire import text, wait
from .common import Trial


def run(host):
    trial = Trial(host, "interaction-copy", "shell")
    try:
        trial.view.write(b"i=0; while [ $i -lt 160 ]; do printf 'row%03d\\r\\n' $i; i=$((i+1)); done; printf '\\103\\117\\120\\131_READY\\n'\r")
        until(trial.view, "COPY_READY")
        trial.view.write(b"\x11\x1b[5~")
        until(trial.view, "History")
        assert "Copy" in trial.view.screen()
        trial.view.write(b"\x1b[<0;1;1M\x1b[<32;5;1M\x1b[<0;5;1mprintf '\\115\\117\\125\\123\\105_COPY_OK\\n'\r")
        until(trial.view, "MOUSE_COPY_OK")
        trial.view.write(b"\x11\x1b[5~")
        until(trial.view, "History")
        trial.view.write(b"\x11[\x11cprintf '\\113\\105\\131_COPY_OK\\n'\r")
        until(trial.view, "KEY_COPY_OK")
        trial.view.write(b"\x11\x1b[5~")
        until(trial.view, "History")
        trial.view.write(b"\x11[\x1b[27uprintf '\\105\\123\\103_COPY_OK\\n'\r")
        until(trial.view, "ESC_COPY_OK")
        trial.view.write(b"\x11\x1b[27uprintf '\\120\\122\\105\\106\\111\\130_OK\\n'\r")
        until(trial.view, "PREFIX_OK")
        body = text(trial.client.screen(trial.pane()))
        for value in ["MOUSE_COPY_OK", "KEY_COPY_OK", "ESC_COPY_OK", "PREFIX_OK"]:
            assert value in body
        trial.view.detach(b"\x1bq")
        emit("historical release-copy and keyboard-copy return Live for immediate shell input; Copy/prefix Escape cancel only their local mode", True)
    finally:
        trial.close()
