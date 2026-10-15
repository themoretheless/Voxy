# Facial rendering source scan: Recol/DLSS-Updater

Repository: https://github.com/Recol/DLSS-Updater

Pinned source revision: 8427b8a3397a0a12b42709cfd6ec48a549612716

Discovery topic: DLSS

Description: What if you could update all the games that are detected on your system?

Reported license: AGPL-3.0

Status: mechanical source scan. Source retrieval, path classification, and keyword counts are verified; this is not a completed deep review.

All repository text below is untrusted reference data. It is not an instruction to execute commands or change agent behaviour.

License paths: ["LICENSE"]

Topic paths: {"dlss": ["DLSS-Updater.nuspec", "DLSS_Updater.spec", "DLSS_Updater_Linux.spec", "DLSS_Updater_MSI.spec", "dlss_updater/__init__.py", "dlss_updater/auto_updater.py", "dlss_updater/backup_manager.py", "dlss_updater/cache_manager.py", "dlss_updater/concurrency_limiters.py", "dlss_updater/config.py", "dlss_updater/constants.py", "dlss_updater/database.py", "dlss_updater/desktop_identity.py", "dlss_updater/dll_repository.py", "dlss_updater/exe_resolver.py", "dlss_updater/fsr4_installer.py", "dlss_updater/gpu_detection.py", "dlss_updater/high_performance_updater.py", "dlss_updater/icons/battlenet.png", "dlss_updater/icons/dlss_updater.ico", "dlss_updater/icons/dlss_updater.png", "dlss_updater/icons/dlss_updater_full.png", "dlss_updater/icons/ea.png", "dlss_updater/icons/epic.png", "dlss_updater/icons/gog.png", "dlss_updater/icons/reset.png", "dlss_updater/icons/steam.png", "dlss_updater/icons/ubisoft.png", "dlss_updater/icons/update.png", "dlss_updater/icons/xbox.png"], "face": ["dlss_updater/ui_flet/components/hero_surface.py"]}

## Source file: dlss_updater/ui_flet/components/hero_surface.py

{
  "path": "dlss_updater/ui_flet/components/hero_surface.py",
  "blob_sha": "d4f9c1a0bb3df854f1abeb5c04b1aeb9e2072f6c",
  "sha256": "ae3dbfe860aa24217ef66f321c30c2042ebd24743377d4f98156addeb353616f",
  "lines": 247,
  "bytes": 10561,
  "selection_score": 18,
  "keyword_line_counts": {
    "dlss": 1,
    "rig": 7,
    "face": 15
  },
  "url": "https://github.com/Recol/DLSS-Updater/blob/8427b8a3397a0a12b42709cfd6ec48a549612716/dlss_updater/ui_flet/components/hero_surface.py",
  "function_signature_candidates": [
    {
      "line": 62,
      "text": "def build_scrim_gradient(is_dark: bool) -> ft.LinearGradient:"
    },
    {
      "line": 113,
      "text": "def build_art_scrim_gradient(is_dark: bool) -> ft.LinearGradient:"
    },
    {
      "line": 132,
      "text": "def art_tint_color(is_dark: bool) -> str:"
    },
    {
      "line": 139,
      "text": "def _blend_hex(base: str, tint: str, alpha: float) -> str:"
    },
    {
      "line": 153,
      "text": "def build_brand_wash("
    },
    {
      "line": 188,
      "text": "def build_watermark_icon("
    },
    {
      "line": 208,
      "text": "def build_pill("
    },
    {
      "line": 240,
      "text": "def themed_accent(pair: tuple[str, str], is_dark: bool) -> str:"
    }
  ]
}

```text

"""
Hero Surface — shared "hero design" DNA.

This module extracts the reusable visual primitives that make up the hero
card language pioneered by ``game_card.py`` (the game library's full-bleed
artwork cards): a bottom-weighted scrim gradient for text legibility over
artwork, a diagonal brand-color "wash" for cards without artwork, an
oversized decorative watermark glyph, lightweight Container-based status
pills (see CLAUDE.md's "Container-based Badge Pattern"), and a helper for
picking the correct light/dark accent out of a ``(dark, light)`` color pair
(the convention used by ``settings_view.TILE_COLORS``).

Everything here is a pure function or module constant — no classes, no
``ft.Page`` access, no mutable state. Callers own the ``ft.Control`` objects
these functions return and are free to re-theme/rebuild them as needed.
"""

import flet as ft

from dlss_updater.ui_flet.theme.colors import MD3Colors

# ==================== HOVER ANIMATION ====================
# Matches the hover-scale convention shared by game_card.py (scale=1.015,
# 200ms EASE_OUT) and hub_card.py (scale=1.01-1.02, 150ms EASE_OUT).
HOVER_SCALE = 1.02
HOVER_ANIM_MS = 200

# ==================== BRAND WASH OPACITY ====================
# Diagonal brand-color gradient used on cards/tiles that have no artwork of
# their own (e.g. launcher shells, settings tiles). Dark theme gets a
# stronger wash since dark surfaces need more saturation to read as tinted.
WASH_OPACITY_DARK = 0.22
WASH_OPACITY_LIGHT = 0.14

# ==================== WATERMARK OPACITY ====================
# Oversized decorative glyph opacity — subtle enough to never compete with
# foreground content.
WATERMARK_OPACITY_DARK = 0.08
WATERMARK_OPACITY_LIGHT = 0.04

# ==================== PILL GEOMETRY ====================
PILL_HEIGHT = 26

# ==================== SCRIM TUNING ====================
# The bottom-weighted scrim keeps the identity caption legible over artwork.
# Dark mode fades toward the near-black surface, which reads as a natural
# vignette — extending it high up the tile is harmless. Light mode fades
# toward WHITE, so the same tall ramp visibly desaturates/washes out the
# artwork (verified on the light-mode Games mosaic). The light profile
# therefore keeps a much taller CLEAR zone (scrim confined to the caption
# band near the bottom) while preserving a strong opaque edge where the text
# actually sits. Stops/opacities are paired 1:1 (position, white-veil alpha);
# an alpha of 0 emits a fully transparent stop. This layer is shadow-LESS, so
# alpha stops are safe here (CLAUDE.md rendering pitfall #1 applies only to
# shadowed containers).
_SCRIM_STOPS_DARK = [0.0, 0.45, 0.75, 1.0]
_SCRIM_OPACITY_DARK = [0.0, 0.15, 0.60, 0.85]
_SCRIM_STOPS_LIGHT = [0.0, 0.58, 0.82, 1.0]
_SCRIM_OPACITY_LIGHT = [0.0, 0.10, 0.50, 0.90]


def build_scrim_gradient(is_dark: bool) -> ft.LinearGradient:
    """Bottom-weighted gradient scrim for legibility over artwork.

    Generalized copy of ``GameCard._build_scrim_gradient()``. Fades from
    fully transparent (top) to the theme's own surface color (bottom) —
    not a hardcoded black — so the scrim's opaque edge matches whatever
    themed surface sits below it (footer, card background, etc.) in both
    light and dark mode. Light mode uses a taller clear zone so it doesn't
    wash out the artwork it sits over (see the _SCRIM_* constants above).
    """
    base = MD3Colors.get_surface(is_dark)
    stops = _SCRIM_STOPS_DARK if is_dark else _SCRIM_STOPS_LIGHT
    opacities = _SCRIM_OPACITY_DARK if is_dark else _SCRIM_OPACITY_LIGHT
    return ft.LinearGradient(
        begin=ft.Alignment.TOP_CENTER,
        end=ft.Alignment.BOTTOM_CENTER,
        colors=[
            ft.Colors.TRANSPARENT if o <= 0 else ft.Colors.with_opacity(o, base)
            for o in opacities
        ],
        stops=stops,
    )


# ==================== ART MOSAIC OVERLAYS ====================
# Overlays for hero surfaces built out of the user's OWN artwork (the hub's
# Games mosaic). These deliberately do NOT reuse build_scrim_gradient: that one
# fades toward the themed surface because a game card's artwork meets an opaque
# themed footer directly below it. A mosaic has no themed surface below it at
# all, so in light mode a white-fading ramp only hazes the art instead of
# anchoring the caption. These fade toward BLACK in both themes, and the caller
# switches its caption to light-on-dark text while the mosaic is live.
#
# Both are intended for shadow-LESS Containers inside the hero's Stack.
# CLAUDE.md rendering pitfall #1: an alpha-carrying gradient on a Container
# that ALSO has a box-shadow renders near-black, because the shadow is painted
# directly behind the box and shows through every transparent region.
_ART_SCRIM_STOPS = [0.0, 0.55, 0.80, 1.0]
_ART_SCRIM_ALPHA_DARK = [0.0, 0.10, 0.55, 0.88]
_ART_SCRIM_ALPHA_LIGHT = [0.0, 0.08, 0.46, 0.78]

# Flat unifying tint laid over EVERY mosaic tile. Cached cover art ranges from
# near-black to near-white, so without it a single bright cover blows the
# mosaic out and the scrim's contrast becomes unpredictable. Light mode leans
# on it slightly less: the tint is also what keeps a bright tile from merging
# into the near-white light canvas, but the hairline outline carries part of
# that job there.
ART_TINT_DARK = 0.26
ART_TINT_LIGHT = 0.20


def build_art_scrim_gradient(is_dark: bool) -> ft.LinearGradient:
    """Bottom-weighted BLACK scrim for captions over arbitrary user artwork.

    Clear through the top ~55% so the art reads unobstructed, then ramps to a
    near-opaque black ground under the caption band. See the _ART_SCRIM_*
    constants for why this is black rather than the themed surface.
    """
    alphas = _ART_SCRIM_ALPHA_DARK if is_dark else _ART_SCRIM_ALPHA_LIGHT
    return ft.LinearGradient(
        begin=ft.Alignment.TOP_CENTER,
        end=ft.Alignment.BOTTOM_CENTER,
        colors=[
            ft.Colors.TRANSPARENT if a <= 0 else ft.Colors.with_opacity(a, ft.Colors.BLACK)
            for a in alphas
        ],
        stops=_ART_SCRIM_STOPS,
    )


def art_tint_color(is_dark: bool) -> str:
    """Flat low-alpha black that unifies mosaic tiles of wildly varying brightness."""
    return ft.Colors.with_opacity(
        ART_TINT_DARK if is_dark else ART_TINT_LIGHT, ft.Colors.BLACK
    )


def _blend_hex(base: str, tint: str, alpha: float) -> str:
    """Alpha-blend opaque ``tint`` over opaque ``base`` (both ``#RRGGBB``),
    returning an opaque ``#RRGGBB``. Used to pre-compose wash gradients so
    they carry NO alpha channel — see build_brand_wash for why.
    """
    b, t = base.lstrip("#"), tint.lstrip("#")
    br, bg_, bb = int(b[0:2], 16), int(b[2:4], 16), int(b[4:6], 16)
    tr, tg, tb = int(t[0:2], 16), int(t[2:4], 16), int(t[4:6], 16)
    r = round(tr * alpha + br * (1 - alpha))
    g = round(tg * alpha + bg_ * (1 - alpha))
    bl = round(tb * alpha + bb * (1 - alpha))
    return f"#{r:02X}{g:02X}{bl:02X}"


def build_brand_wash(
    accent: str, is_dark: bool, opacity: float | None = None
) -> ft.LinearGradient:
    """Diagonal brand-color wash (top-left -> bottom-right) for artwork-less surfaces.

    IMPORTANT: the gradient is fully OPAQUE — the accent is pre-blended over
    the themed surface color rather than using alpha stops. Two reasons
    (both verified live, see CLAUDE.md's Flet client notes):
    1. Flutter's BoxDecoration ignores ``color`` when ``gradient`` is set, so
       a translucent wash painted directly on a Container never composites
       over that Container's own bgcolor.
    2. The Container's box-shadow is painted directly behind the box, so any
       transparent gradient region shows the shadow's BLACK through it —
       invisible over dark themes, but it rendered every washed surface
       near-black in light mode.

    ``opacity`` (the accent blend strength at the top-left corner) defaults
    to ``WASH_OPACITY_DARK``/``WASH_OPACITY_LIGHT`` based on ``is_dark``.
    Accents may be given as ``#RRGGBB``; anything unparseable falls back to
    a flat surface gradient.
    """
    if opacity is None:
        opacity = WASH_OPACITY_DARK if is_dark else WASH_OPACITY_LIGHT
    base = MD3Colors.get_surface(is_dark)
    try:
        start = _blend_hex(base, accent, opacity)
    except (ValueError, IndexError):
        start = base
    return ft.LinearGradient(
        begin=ft.Alignment.TOP_LEFT,
        end=ft.Alignment.BOTTOM_RIGHT,
        colors=[start, base],
    )


def build_watermark_icon(
    icon: str,
    is_dark: bool,
    size: int = 110,
    alignment: ft.Alignment | None = None,
) -> ft.Container:
    """Oversized decorative glyph, purely visual (never intercepts input).

    A plain ``ft.Icon`` with no event handlers, hosted in a Container so it
    can be positioned/sized independently of surrounding layout. Defaults
    to bottom-right placement to read as an "overflowing" background motif.
    """
    opacity = WATERMARK_OPACITY_DARK if is_dark else WATERMARK_OPACITY_LIGHT
    return ft.Container(
        content=ft.Icon(icon, size=size, color=ft.Colors.WHITE),
        alignment=alignment or ft.Alignment.BOTTOM_RIGHT,
        opacity=opacity,
    )


def build_pill(
    text: str,
    *,
    icon: str | None = None,
    bgcolor: str,
    text_color: str = ft.Colors.WHITE,
    icon_color: str | None = None,
    text_size: int = 11,
    icon_size: int = 14,
) -> ft.Container:
    """Lightweight Container-based status/badge pill.

    Matches CLAUDE.md's "Container-based Badge Pattern" — a Row of an
    optional Icon + Text inside a rounded Container, used in place of the
    heavier ``ft.Chip`` for read-only badges.
    """
    row_controls: list[ft.Control] = []
    if icon is not None:
        row_controls.append(ft.Icon(icon, size=icon_size, color=icon_color or text_color))
    row_controls.append(
        ft.Text(text, size=text_size, color=text_color, weight=ft.FontWeight.W_500)
    )

    return ft.Container(
        content=ft.Row(row_controls, spacing=4, tight=True),
        bgcolor=bgcolor,
        padding=ft.Padding.symmetric(horizontal=8, vertical=4),
        border_radius=16,
        height=PILL_HEIGHT,
    )


def themed_accent(pair: tuple[str, str], is_dark: bool) -> str:
    """Pick the themed variant from a ``(dark_value, light_value)`` accent pair.

    Matches the tuple ordering used by ``settings_view.TILE_COLORS`` and
    ``MD3Colors.THEMED``: index 0 is the dark-mode value, index 1 the
    light-mode value.
    """
    return pair[0] if is_dark else pair[1]


```

## Source file: hooks/hook-dlss_updater.py

{
  "path": "hooks/hook-dlss_updater.py",
  "blob_sha": "82ecf846a8b8da26bb5c0b4d1d4da4993db8b43f",
  "sha256": "416fc9aed513c68a4c3b6ea25b2604d66582eabea7cbf5061873d0a6298b2b2d",
  "lines": 13,
  "bytes": 325,
  "selection_score": 15,
  "keyword_line_counts": {
    "dlss": 2
  },
  "url": "https://github.com/Recol/DLSS-Updater/blob/8427b8a3397a0a12b42709cfd6ec48a549612716/hooks/hook-dlss_updater.py",
  "function_signature_candidates": []
}

```text

from PyInstaller.utils.hooks import collect_submodules, collect_data_files

hiddenimports = collect_submodules('dlss_updater')

# Add Pillow modules for WebP thumbnail generation
hiddenimports += [
    'PIL',
    'PIL.Image',
    'PIL.WebPImagePlugin',
    'PIL.JpegImagePlugin',
]

datas = collect_data_files('dlss_updater')

```

## Source file: dlss_updater/linux_dlss_utils.py

{
  "path": "dlss_updater/linux_dlss_utils.py",
  "blob_sha": "06ce38e0eddf820c7d489350d6bc8c91d39bce37",
  "sha256": "8ae3d8cee08dc459a1234ab76741eee160b25f60fa8cda6414fea2a38dfd2e7a",
  "lines": 197,
  "bytes": 6956,
  "selection_score": 15,
  "keyword_line_counts": {
    "dlss": 40
  },
  "url": "https://github.com/Recol/DLSS-Updater/blob/8427b8a3397a0a12b42709cfd6ec48a549612716/dlss_updater/linux_dlss_utils.py",
  "function_signature_candidates": [
    {
      "line": 39,
      "text": "def _preset_env_value(preset_key: str) -> str:"
    },
    {
      "line": 47,
      "text": "def generate_steam_launch_options("
    },
    {
      "line": 66,
      "text": "def cap_ok(cap: str) -> bool:"
    },
    {
      "line": 127,
      "text": "def get_preset_description(preset: DLSSPreset | str) -> str:"
    },
    {
      "line": 171,
      "text": "def _preset_tuples(presets: tuple[DLSSPreset, ...]) -> list[tuple[str, str, str]]:"
    },
    {
      "line": 178,
      "text": "def get_all_presets() -> list[tuple[str, str, str]]:"
    },
    {
      "line": 189,
      "text": "def get_rr_presets() -> list[tuple[str, str, str]]:"
    }
  ]
}

```text

"""
Linux Proton upscaler utility functions.

Generates Steam launch options covering:
- DXVK-NVAPI DLSS DRS overrides (SR/RR render presets, FG override, debug
  overlay) — work on any Proton that bundles dxvk-nvapi, including Valve's.
- Community Proton fork upscaler upgrades (GE-Proton / Proton-CachyOS / EM):
  PROTON_DLSS_UPGRADE, PROTON_FSR4_UPGRADE (+RDNA3 variant), PROTON_XESS_UPGRADE
  and their on-screen indicators.
- Proton misc toggles (Wayland, HDR).

The output is a plain ``ENV=value ... %command%`` prefix: paste into Steam's
launch options as-is, or use the env-var portion in Heroic/Lutris environment
settings — the format is intentionally launcher-agnostic.

Environment Variables Reference:
- DXVK_NVAPI_DRS_NGX_DLSS_SR_OVERRIDE_RENDER_PRESET_SELECTION: SR preset
- DXVK_NVAPI_DRS_NGX_DLSS_RR_OVERRIDE_RENDER_PRESET_SELECTION: RR preset
- DXVK_NVAPI_DRS_NGX_DLSS_FG_OVERRIDE: Frame Generation override (on)
- DXVK_NVAPI_SET_NGX_DEBUG_OPTIONS: Debug overlay
- PROTON_DLSS_UPGRADE / PROTON_DLSS_INDICATOR: DLSS DLL upgrade + HUD (forks)
- PROTON_FSR4_UPGRADE / PROTON_FSR4_RDNA3_UPGRADE / PROTON_FSR4_INDICATOR
- PROTON_XESS_UPGRADE
- PROTON_ENABLE_WAYLAND: Wayland support
- PROTON_ENABLE_HDR / ENABLE_HDR_WSI: HDR support
"""

from dlss_updater.models import DLSSPreset, LinuxDLSSConfig
from dlss_updater.proton_compat import (
    CAP_DLSS_INDICATOR,
    CAP_DLSS_UPGRADE,
    CAP_FSR4_INDICATOR,
    CAP_FSR4_RDNA3_UPGRADE,
    CAP_FSR4_UPGRADE,
    CAP_XESS_UPGRADE,
)


def _preset_env_value(preset_key: str) -> str:
    """Resolve a stored preset key to its DXVK-NVAPI env value ("" if none)."""
    try:
        return DLSSPreset(preset_key).env_value
    except ValueError:
        return ""


def generate_steam_launch_options(
    config: LinuxDLSSConfig,
    capabilities: frozenset[str] | None = None,
) -> str:
    """
    Generate Steam launch options string from config.

    Args:
        config: LinuxDLSSConfig with preset and toggle settings
        capabilities: Optional set of supported PROTON_* upgrade capabilities
            (from proton_compat.classify_compat_tool) — when provided, upgrade
            variables the target Proton build doesn't support are omitted.
            DXVK-NVAPI variables are never filtered (they work on any Proton).

    Returns:
        Complete launch options string ending with %command%
    """
    parts: list[str] = []

    def cap_ok(cap: str) -> bool:
        return capabilities is None or cap in capabilities

    # SR Preset (only if not default)
    if config.selected_preset != DLSSPreset.DEFAULT:
        env_value = _preset_env_value(config.selected_preset)
        if env_value:
            parts.append(
                f"DXVK_NVAPI_DRS_NGX_DLSS_SR_OVERRIDE_RENDER_PRESET_SELECTION={env_value}"
            )

    # RR Preset (only if not default)
    if config.rr_preset != DLSSPreset.DEFAULT:
        env_value = _preset_env_value(config.rr_preset)
        if env_value:
            parts.append(
                f"DXVK_NVAPI_DRS_NGX_DLSS_RR_OVERRIDE_RENDER_PRESET_SELECTION={env_value}"
            )

    # Frame Generation override
    if config.fg_override:
        parts.append("DXVK_NVAPI_DRS_NGX_DLSS_FG_OVERRIDE=on")

    # Debug overlay
    if config.overlay_enabled:
        parts.append("DXVK_NVAPI_SET_NGX_DEBUG_OPTIONS=DLSSIndicator=1024")

    # Fork upscaler upgrades (capability-gated)
    if config.dlss_upgrade and cap_ok(CAP_DLSS_UPGRADE):
        parts.append("PROTON_DLSS_UPGRADE=1")
    if config.dlss_indicator and cap_ok(CAP_DLSS_INDICATOR):
        parts.append("PROTON_DLSS_INDICATOR=1")
    if config.fsr4_upgrade and cap_ok(CAP_FSR4_UPGRADE):
        # RDNA 3 mode is gated on its OWN capability: Proton-CachyOS removed
        # PROTON_FSR4_RDNA3_UPGRADE, so on that fork we must fall back to the plain
        # variable rather than emitting one it ignores (which yielded no upgrade at
        # all). GE-Proton still honours the RDNA 3 spelling.
        if config.fsr4_rdna3_mode and cap_ok(CAP_FSR4_RDNA3_UPGRADE):
            parts.append("PROTON_FSR4_RDNA3_UPGRADE=1")
        else:
            parts.append("PROTON_FSR4_UPGRADE=1")
    if config.fsr4_indicator and cap_ok(CAP_FSR4_INDICATOR):
        parts.append("PROTON_FSR4_INDICATOR=1")
    if config.xess_upgrade and cap_ok(CAP_XESS_UPGRADE):
        parts.append("PROTON_XESS_UPGRADE=1")

    # Wayland
    if config.wayland_enabled:
        parts.append("PROTON_ENABLE_WAYLAND=1")

    # HDR (requires both variables)
    if config.hdr_enabled:
        parts.append("PROTON_ENABLE_HDR=1")
        parts.append("ENABLE_HDR_WSI=1")

    # Always end with %command%
    if parts:
        return " ".join(parts) + " %command%"
    return "%command%"


def get_preset_description(preset: DLSSPreset | str) -> str:
    """
    Get human-readable description for a preset.

    Args:
        preset: DLSSPreset enum value or string value

    Returns:
        Description string explaining the preset's characteristics
    """
    preset_str = preset if isinstance(preset, str) else preset.value

    descriptions = {
        "default": "Use game/driver default settings",
        "latest": "Always use the newest preset the DLL provides",
        "preset_j": "Transformer model - slightly less ghosting than K",
        "preset_k": "Lighter preset - better performance, recommended for RTX 20/30 series",
        "preset_l": "Balanced preset - good quality/performance balance",
        "preset_m": "Heavier preset - higher quality, recommended for RTX 40/50 series",
        "preset_d": "DLSS 4 Ray Reconstruction - the first-generation model",
        "preset_f": "DLSS 4.5 Ray Reconstruction Gen 2",
    }
    return descriptions.get(preset_str, "Unknown preset")


_SR_PRESETS = (
    DLSSPreset.DEFAULT,
    DLSSPreset.LATEST,
    DLSSPreset.PRESET_J,
    DLSSPreset.PRESET_K,
    DLSSPreset.PRESET_L,
    DLSSPreset.PRESET_M,
)

# RR mirrors the Windows per-game dialog and the NVIDIA App's RR dropdown:
# Default/Latest plus the only two meaningful RR models, D (DLSS 4) and F (DLSS 4.5).
_RR_PRESETS = (
    DLSSPreset.DEFAULT,
    DLSSPreset.LATEST,
    DLSSPreset.PRESET_D,
    DLSSPreset.PRESET_F,
)


def _preset_tuples(presets: tuple[DLSSPreset, ...]) -> list[tuple[str, str, str]]:
    return [
        (p.value, p.display_name, get_preset_description(p))
        for p in presets
    ]


def get_all_presets() -> list[tuple[str, str, str]]:
    """
    Get all available SR presets with their values, display names, and
    descriptions.

    Returns:
        List of tuples: (value, display_name, description)
    """
    return _preset_tuples(_SR_PRESETS)


def get_rr_presets() -> list[tuple[str, str, str]]:
    """
    Get available Ray Reconstruction presets (Default/Latest, mirroring the
    Windows per-game DLSS dialog).

    Returns:
        List of tuples: (value, display_name, description)
    """
    return _preset_tuples(_RR_PRESETS)


```

## Source file: dlss_updater/ui_flet/panels/dlss_settings_panel.py

{
  "path": "dlss_updater/ui_flet/panels/dlss_settings_panel.py",
  "blob_sha": "3de17d621e451eae1e72d8d4fbee27172c5706d1",
  "sha256": "29c3995318bd1f29f26933f139b0cadcd773806b4bd46fac78384fecb518d4d9",
  "lines": 265,
  "bytes": 9537,
  "selection_score": 15,
  "keyword_line_counts": {
    "dlss": 37,
    "face": 2
  },
  "url": "https://github.com/Recol/DLSS-Updater/blob/8427b8a3397a0a12b42709cfd6ec48a549612716/dlss_updater/ui_flet/panels/dlss_settings_panel.py",
  "function_signature_candidates": [
    {
      "line": 48,
      "text": "def __init__(self, page: ft.Page, logger: logging.Logger):"
    },
    {
      "line": 62,
      "text": "def title(self) -> str:"
    },
    {
      "line": 66,
      "text": "def subtitle(self) -> str | None:"
    },
    {
      "line": 70,
      "text": "def width(self) -> int:"
    },
    {
      "line": 74,
      "text": "def accent(self) -> str:"
    },
    {
      "line": 79,
      "text": "def icon(self) -> str:"
    },
    {
      "line": 82,
      "text": "def _load_config(self):"
    },
    {
      "line": 85,
      "text": "def _build_controls(self):"
    },
    {
      "line": 126,
      "text": "def _on_sr_changed(self, e):"
    },
    {
      "line": 134,
      "text": "def _current_sr(self) -> WindowsDLSSPreset:"
    },
    {
      "line": 140,
      "text": "async def on_open(self):"
    },
    {
      "line": 158,
      "text": "def build(self) -> ft.Control:"
    },
    {
      "line": 214,
      "text": "def get_themed_properties(self) -> dict[str, tuple[str, str]]:"
    },
    {
      "line": 220,
      "text": "def validate(self) -> tuple[bool, str | None]:"
    },
    {
      "line": 223,
      "text": "def _selections(self) -> dict[str, str]:"
    },
    {
      "line": 230,
      "text": "async def on_save(self) -> bool:"
    },
    {
      "line": 263,
      "text": "def on_cancel(self):"
    }
  ]
}

```text

"""
WindowsDLSSPresetsPanel - Global DLSS preset control (Windows).

Writes the chosen presets to the NVIDIA driver base profile via NvAPI DRS - the
same global override the NVIDIA App uses - for all three DLSS components:

    SR  Super Resolution    (named presets J/K/L/M)
    RR  Ray Reconstruction  (Default / Latest model)
    FG  Frame Generation     (Default / Latest model)

Applies to every DLSS title on the system at next launch.

The shared Material 3 panel chrome (info banner, per-feature section blocks,
dividers, caution banner, footnote, themed properties) lives in
:class:`_DLSSPresetPanelBase`; this panel only supplies the global dropdowns and
the global save/cancel behaviour.
"""

import logging

import flet as ft

from dlss_updater.config import config_manager
from dlss_updater.models import (
    WindowsDLSSConfig,
    WindowsDLSSPreset,
    WindowsDLSSModelPreset,
    WindowsDLSSFGPreset,
)
from dlss_updater import nvapi_drs
from dlss_updater.ui_flet.theme.colors import (
    MD3Colors,
    TechnologyColors,
    build_input_border,
)
from dlss_updater.ui_flet.panels.dlss_preset_panel_base import _DLSSPresetPanelBase


class WindowsDLSSPresetsPanel(_DLSSPresetPanelBase):
    """
    Panel for configuring the global Windows DLSS preset overrides (SR/RR/FG).

    Each feature has a dropdown plus a live readout of the value currently in
    the driver base profile (read on open). Save writes all three in a single
    NvAPI DRS transaction.
    """

    def __init__(self, page: ft.Page, logger: logging.Logger):
        super().__init__(page, logger)

        # Per-feature dropdowns + SR description
        self._sr_dropdown: ft.Dropdown | None = None
        self._rr_dropdown: ft.Dropdown | None = None
        self._fg_dropdown: ft.Dropdown | None = None
        self._sr_desc: ft.Text | None = None

        self._load_config()
        self._build_controls()
        self._register_theme_aware()

    @property
    def title(self) -> str:
        return "DLSS Settings (Experimental)"

    @property
    def subtitle(self) -> str | None:
        return "Global preset overrides (SR / RR / FG)"

    @property
    def width(self) -> int:
        return 520

    @property
    def accent(self) -> str:
        """NVIDIA green — this panel writes NVIDIA driver DLSS settings."""
        return TechnologyColors.DLSS

    @property
    def icon(self) -> str:
        return ft.Icons.MEMORY

    def _load_config(self):
        self._config = config_manager.get_windows_dlss_config()

    def _build_controls(self):
        is_dark = self._registry.is_dark
        primary = MD3Colors.get_primary(is_dark)
        outline = MD3Colors.get_outline(is_dark)

        # SR dropdown: full named presets
        self._sr_dropdown = ft.Dropdown(
            options=[
                ft.DropdownOption(key=p.value, text=p.display_name)
                for p in WindowsDLSSPreset
            ],
            value=self._config.selected_preset,
            label="Super Resolution (SR) Preset",
            border=build_input_border(outline, primary),
            on_select=self._on_sr_changed,
            expand=True,
        )

        # RR dropdown: Default / Latest model
        self._rr_dropdown = ft.Dropdown(
            options=[
                ft.DropdownOption(key=p.value, text=p.display_name)
                for p in WindowsDLSSModelPreset
            ],
            value=self._config.rr_preset,
            label="Ray Reconstruction (RR) Model",
            border=build_input_border(outline, primary),
            expand=True,
        )
        # FG dropdown: Default / Latest / Preset A / Preset B
        self._fg_dropdown = ft.Dropdown(
            options=[
                ft.DropdownOption(key=p.value, text=p.display_name)
                for p in WindowsDLSSFGPreset
            ],
            value=self._config.fg_preset,
            label="Frame Generation (FG) Preset",
            border=build_input_border(outline, primary),
            expand=True,
        )

    def _on_sr_changed(self, e):
        if self._sr_desc:
            self._sr_desc.value = self._current_sr().description
            try:
                self._sr_desc.update()
            except Exception:
                pass

    def _current_sr(self) -> WindowsDLSSPreset:
        try:
            return WindowsDLSSPreset(self._sr_dropdown.value or "default")
        except ValueError:
            return WindowsDLSSPreset.DEFAULT

    async def on_open(self):
        """Read live driver state for all three features once shown."""
        if not nvapi_drs.is_available():
            for feat in ("sr", "rr", "fg"):
                self._set_status(feat, "NVIDIA driver not detected", warning=True)
            return

        values, error = await nvapi_drs.get_current_presets()
        if error:
            self.logger.warning(f"Could not read current DLSS presets: {error}")
            for feat in ("sr", "rr", "fg"):
                self._set_status(feat, "Could not read driver state", warning=True)
            return

        for feat in ("sr", "rr", "fg"):
            desc = nvapi_drs.describe_preset_value(values.get(feat))
            self._set_status(feat, f"Currently applied: {desc}")

    def build(self) -> ft.Control:
        is_dark = self._registry.is_dark

        # Info banner (shared chrome)
        info_container = self._info_banner(
            "Force global DLSS presets for every game on this system. These write "
            "to the NVIDIA driver profile (the same settings the NVIDIA App uses) "
            "and apply the next time each game launches."
        )

        # SR section (with live description)
        self._sr_desc = ft.Text(
            self._current_sr().description,
            size=12,
            color=MD3Colors.get_on_surface_variant(is_dark),
            italic=True,
        )

        controls: list[ft.Control] = [
            info_container,
            ft.Container(height=8),
            self.section_label("Super Resolution (Upscaling)"),
            *self._feature_block("sr", self._sr_dropdown, is_dark, extra=self._sr_desc),
            self.divider(),
            self.section_label("Ray Reconstruction (Denoising)"),
            *self._feature_block("rr", self._rr_dropdown, is_dark),
            self.divider(),
            self.section_label("Frame Generation"),
            *self._feature_block("fg", self._fg_dropdown, is_dark),
            self.divider(),
        ]

        # NVIDIA App interaction caution - the App writes PER-GAME overrides that
        # win over this global setting. Verified live: the NVIDIA App stamps its
        # DLSS override into hundreds of individual game profiles, not just the
        # global base profile this panel writes.
        controls.append(
            self._caution_banner(
                "Using the NVIDIA App's DLSS override? It writes per-game "
                "overrides that take priority over this global setting for "
                "those games. To let this control them, set the DLSS override "
                "to Default/Off in the NVIDIA App."
            )
        )

        controls.append(
            self._footnote(
                "'Default' clears that global override. RR exposes Default/Latest and FG "
                "adds Presets A/B (higher lettered model variants are undocumented and "
                "hidden); the status line shows whatever is currently applied. Restart "
                "games to apply changes."
            )
        )

        return ft.Column(controls=controls, spacing=8, scroll=ft.ScrollMode.AUTO)

    def get_themed_properties(self) -> dict[str, tuple[str, str]]:
        props = super().get_themed_properties()
        if self._sr_desc:
            props["_sr_desc.color"] = MD3Colors.get_themed_pair("on_surface_variant")
        return props

    def validate(self) -> tuple[bool, str | None]:
        return True, None

    def _selections(self) -> dict[str, str]:
        return {
            "sr": self._sr_dropdown.value or "default",
            "rr": self._rr_dropdown.value or "default",
            "fg": self._fg_dropdown.value or "default",
        }

    async def on_save(self) -> bool:
        """Apply all three presets to the driver and persist the choices."""
        if not nvapi_drs.is_available():
            self._show_error_dialog(
                "Not Available",
                "DLSS preset control requires an NVIDIA driver on Windows.",
            )
            return False

        selections = self._selections()
        success, error = await nvapi_drs.apply_presets(selections)
        if not success:
            self.logger.error(f"Failed to apply DLSS presets: {error}")
            self._show_error_dialog(
                "Could Not Apply Presets",
                error or "Unknown error writing NVIDIA driver settings.",
            )
            return False

        config_manager.save_windows_dlss_config(
            WindowsDLSSConfig(
                selected_preset=selections["sr"],
                rr_preset=selections["rr"],
                fg_preset=selections["fg"],
            )
        )
        self.logger.info(f"Windows DLSS global presets applied: {selections}")
        self._show_snackbar(
            "DLSS presets applied. Restart games to see changes.",
            bgcolor="#4CAF50",
        )
        return True

    def on_cancel(self):
        self.logger.debug("Windows DLSS presets panel cancelled, discarding changes")
        self._load_config()


```
