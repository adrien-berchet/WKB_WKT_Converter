from typing import Literal, Optional, Tuple, Union

# Runtime rejects srid=True and integers outside the u32 range. Type checkers
# cannot precisely express those constraints while still accepting int SRIDs.
_SridArg = Union[None, Literal[False], int]


def wkb_to_wkt(wkb: bytes, srid: _SridArg = None) -> str: ...


def wkb_to_wkt_split_srid(wkb: bytes) -> Tuple[str, Optional[int]]: ...


def wkt_to_wkb(wkt: str, srid: _SridArg = None) -> bytes: ...


def wkt_to_wkb_split_srid(wkt: str) -> Tuple[bytes, Optional[int]]: ...


def wkt_to_hex_wkb(wkt: str, srid: _SridArg = None) -> str: ...


def hex_wkb_to_wkt(hex: str, srid: _SridArg = None) -> str: ...


def text_to_wkb(text: str, srid: _SridArg = None) -> bytes: ...


def text_to_wkt(
    text: str,
    srid: _SridArg = None,
    normalize_wkt: bool = False,
) -> str: ...


def text_to_hex_wkb(text: str, srid: _SridArg = None) -> str: ...
