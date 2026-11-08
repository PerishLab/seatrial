import codecs
import re
import unicodedata


class View:
    def __init__(self, columns, rows):
        self.decoder = codecs.getincrementaldecoder("utf-8")()
        self.pending = ""
        self.osc = []
        self.background = None
        self.wrap = False
        self.x = 0
        self.y = 0
        self.resize(columns, rows)

    def resize(self, columns, rows):
        self.columns = columns
        self.rows = rows
        self.cells = [[" "] * columns for _ in range(rows)]
        self.colors = [[None] * columns for _ in range(rows)]
        self.x = min(self.x, columns - 1)
        self.y = min(self.y, rows - 1)

    def feed(self, data):
        self.pending += self.decoder.decode(data)
        while self.pending:
            if self.pending[0] == "\x1b":
                if not self.escape():
                    break
                continue
            value = self.pending[0]
            self.pending = self.pending[1:]
            self.place(value)

    def escape(self):
        if len(self.pending) < 2:
            return False
        if self.pending[1] == "[":
            match = re.match(r"\x1b\[([0-?]*)([ -/]*)([@-~])", self.pending)
            if match is None:
                assert len(self.pending) < 256, "incomplete CSI output"
                return False
            self.csi(match[1], match[3])
            self.pending = self.pending[match.end():]
            return True
        if self.pending[1] == "]":
            return self.string()
        self.pending = self.pending[2:]
        return True

    def string(self):
        endings = [(self.pending.find(value, 2), len(value), value) for value in ["\x07", "\x1b\\", "\x18"]]
        endings = [item for item in endings if item[0] >= 0]
        if not endings:
            assert len(self.pending) <= 2097152, "incomplete OSC output"
            return False
        end, count, value = min(endings)
        if value != "\x18":
            self.osc.append(self.pending[2:end])
        self.pending = self.pending[end + count:]
        return True

    def csi(self, source, action):
        values = [int(value or 0) for value in source.lstrip("?<>=").split(";")]
        value = values[0]
        match action:
            case "H" | "f":
                self.y = min(max(value, 1) - 1, self.rows - 1)
                self.x = min(max(values[1] if len(values) > 1 else 1, 1) - 1, self.columns - 1)
            case "J" if value == 2:
                self.cells = [[" "] * self.columns for _ in range(self.rows)]
                self.colors = [[None] * self.columns for _ in range(self.rows)]
            case "K":
                start = 0 if value in (1, 2) else self.x
                end = self.columns if value in (0, 2) else self.x + 1
                self.cells[self.y][start:end] = [" "] * (end - start)
                self.colors[self.y][start:end] = [self.background] * (end - start)
            case "m":
                self.sgr(values)
            case "h" | "l" if source == "?7":
                self.wrap = action == "h"
            case "h" if source == "?1049":
                self.resize(self.columns, self.rows)

    def sgr(self, values):
        while values:
            value = values.pop(0)
            if value == 0:
                self.background = None
                continue
            if value in (38, 48) and values:
                mode = values.pop(0)
                count = 1 if mode == 5 else 3
                color = values[:count]
                values = values[count:]
                if value == 48:
                    self.background = color[0] if count == 1 else tuple(color)

    def place(self, value):
        match value:
            case "\r":
                self.x = 0
                return
            case "\n":
                self.y += 1
                if self.y == self.rows:
                    self.cells.pop(0)
                    self.cells.append([" "] * self.columns)
                    self.colors.pop(0)
                    self.colors.append([None] * self.columns)
                    self.y -= 1
                return
        if ord(value) < 32 or 127 <= ord(value) < 160:
            return
        if unicodedata.combining(value):
            if self.x:
                self.cells[self.y][self.x - 1] += value
            return
        width = 2 if unicodedata.east_asian_width(value) in ("W", "F") else 1
        if self.x >= self.columns:
            if self.wrap:
                self.x = 0
                self.place("\n")
            else:
                self.x = self.columns - 1
        self.cells[self.y][self.x] = value
        self.colors[self.y][self.x] = self.background
        if width == 2 and self.x + 1 < self.columns:
            self.cells[self.y][self.x + 1] = ""
            self.colors[self.y][self.x + 1] = self.background
        self.x += width

    def text(self):
        return "\n".join("".join(row) for row in self.cells)
