CREATE DATABASE Dex;

CREATE TABLE Auth (
    ID VARCHAR(255),
    Token VARCHAR(255),
    PRIMARY KEY (ID),
    FOREIGN KEY (ID) REFERENCES Dex(ID)
);

CREATE TABLE Dex (
    ID VARCHAR(255),
    Data BLOB(512000),
    PRIMARY KEY (ID)
);

CREATE TABLE DexMetadata (
    PRIMARY KEY (ID),
    Views INT,
    CreationDate DATETIME DEFAULT CURRENT_TIMESTAMP,
    LastViewed DATETIME DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,
    FOREIGN KEY (ID) REFERENCES Dex(ID)
);

CREATE TABLE Passphrases (
    Phrase varchar(255)
);
