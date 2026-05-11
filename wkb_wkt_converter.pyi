from typing import Literal, Optional, Tuple, Union

# Runtime rejects srid=True and integers outside the i32 range. Type checkers
# cannot precisely express those constraints while still accepting int SRIDs.
_SridArg = Union[None, Literal[False], int]
# Runtime also accepts other C-contiguous one-byte buffer-protocol objects, but
# static typing cannot express item size or contiguity for arbitrary exporters.
_WkbInput = Union[bytes, bytearray, memoryview]
_Input = Union[str, _WkbInput]


def wkb_to_wkt(wkb: _WkbInput, srid: _SridArg = None) -> str: ...


def wkb_to_wkt_split_srid(wkb: _WkbInput) -> Tuple[str, Optional[int]]: ...


def wkt_to_wkb(wkt: str, srid: _SridArg = None) -> bytes: ...


def wkt_to_wkb_split_srid(wkt: str) -> Tuple[bytes, Optional[int]]: ...


def wkt_to_hex_wkb(wkt: str, srid: _SridArg = None) -> str: ...


def hex_wkb_to_wkt(hex_wkb: str, srid: _SridArg = None) -> str: ...


def to_wkb(source: _Input, srid: _SridArg = None) -> bytes: ...


def to_wkt(
    source: _Input,
    srid: _SridArg = None,
    normalize_wkt: bool = False,
) -> str: ...


def to_hex_wkb(source: _Input, srid: _SridArg = None) -> str: ...
