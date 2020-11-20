CREATE TABLE ReadingKinds (
    symbol CHAR(1) PRIMARY KEY
);

CREATE TABLE Sensors (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    address CHAR(16) NOT NULL,
    name VARCHAR NULL
);

INSERT INTO ReadingKinds Values ('T'); -- Temperature
INSERT INTO ReadingKinds Values ('H'); -- Humidity

CREATE TABLE Readings (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    sensor INTEGER NOT NULL,
    timestamp DATETIME NOT NULL,
    kind CHAR(1) NOT NULL,
    value INT NOT NULL,
    FOREIGN KEY(sensor) REFERENCES Sensors(id),
    FOREIGN KEY(kind) REFERENCES ReadingKinds(symbol)
);
