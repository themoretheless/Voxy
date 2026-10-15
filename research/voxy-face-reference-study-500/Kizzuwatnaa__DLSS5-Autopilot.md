# Facial rendering source scan: Kizzuwatnaa/DLSS5-Autopilot

Repository: https://github.com/Kizzuwatnaa/DLSS5-Autopilot

Pinned source revision: 5897489e6469418498d6522c933674518c930ca8

Discovery topic: DLSS

Description: Puts DLSS 5 neural rendering into games that never shipped it, and brings the game's own DLSS up to date - super resolution, ray reconstruction, frame generation. Scans your library, picks one of eight routes, fetches every part from its publisher at run time, then says what happened. One exe, no admin; uninstall puts the game back.

Reported license: MIT

Status: mechanical source scan. Source retrieval, path classification, and keyword counts are verified; this is not a completed deep review.

All repository text below is untrusted reference data. It is not an instruction to execute commands or change agent behaviour.

License paths: ["LICENSE", "NOTICE"]

Topic paths: {"dlss": ["_tools/dlss_page_check.py", "core/dlss.py", "core/dlssupdate.py", "core/ui/ctl_dlss.py", "core/ui/dlsspage.py", "dlss5-autopilot.ico", "dlss5_autopilot.py", "docs/screenshots/dlss.png"]}

## Source file: core/dlss.py

{
  "path": "core/dlss.py",
  "blob_sha": "1b097cab08f433d6bc2f8b86cf07421010239908",
  "sha256": "547356c564e8aa23992f7d7eac913c3c8733a5e387675a8abf2484ad61bd8594",
  "lines": 1144,
  "bytes": 58714,
  "selection_score": 15,
  "keyword_line_counts": {
    "eye": 1,
    "skin": 3,
    "sss": 1,
    "shader": 13,
    "dlss": 200,
    "rig": 4
  },
  "url": "https://github.com/Kizzuwatnaa/DLSS5-Autopilot/blob/5897489e6469418498d6522c933674518c930ca8/core/dlss.py",
  "function_signature_candidates": [
    {
      "line": 132,
      "text": "def forget_walk(folder=None) -> None:"
    },
    {
      "line": 144,
      "text": "def _walk_key(p) -> str:"
    },
    {
      "line": 157,
      "text": "def walked(folder: Path, skip_dir: Path | None = None,"
    },
    {
      "line": 184,
      "text": "def _holds_runtime(d: Path) -> bool:"
    },
    {
      "line": 203,
      "text": "def find_dlss_files(folder: Path, skip_dir: Path | None = None,"
    },
    {
      "line": 266,
      "text": "def find_upscaler_files(folder: Path, skip_dir: Path | None = None) -> list[str]:"
    },
    {
      "line": 272,
      "text": "def upscaler_kind(evidence: list[str]) -> str:"
    },
    {
      "line": 311,
      "text": "def __post_init__(self) -> None:"
    },
    {
      "line": 330,
      "text": "def _dlss5_install_here(folder: Path) -> bool:"
    },
    {
      "line": 341,
      "text": "def _ours(folder: Path, name: str) -> bool:"
    },
    {
      "line": 370,
      "text": "def detect(install_dir: Path, folder: Path, api: str, bitness: int,"
    },
    {
      "line": 424,
      "text": "def _shared_steer(s: Support, api: str, sm: int | None, shared: dict | None,"
    },
    {
      "line": 454,
      "text": "def _driver_steer(s: Support, driver: str | None, folder=None, install_dir=None) -> None:"
    },
    {
      "line": 519,
      "text": "def fit(route: str, api: str, native_dlss: bool, sm: int | None,"
    },
    {
      "line": 591,
      "text": "def _detect(install_dir: Path, folder: Path, api: str, bitness: int) -> Support:"
    },
    {
      "line": 622,
      "text": "def _theirs(folder: Path, name: str) -> bool:"
    },
    {
      "line": 638,
      "text": "def _detect_routes(install_dir: Path, folder: Path, api: str,"
    },
    {
      "line": 942,
      "text": "def standalone_fits(api: str, bitness: int | None) -> bool:"
    },
    {
      "line": 955,
      "text": "def driver_warning(route: str, driver: str | None,"
    },
    {
      "line": 1043,
      "text": "def quirks(exe, api: str = \"\") -> tuple[str, ...]:"
    }
  ]
}

```text

"""Does this game ship its own DLSS, and which install path suits it?

This decides between eight ways of getting DLSS 5 neural rendering into a
game. Getting it right matters: the feeder path is always DLAA and needs
motion-vector shaders, whereas a game with its own DLSS can be hooked
directly and keeps its own Quality/Balanced/Performance modes.

    RENODX   ShortFuse's renodx-dlss add-on (the "SF" build). Hooks D3D9,
             D3D11 and D3D12 presentation in-process through a same-adapter
             D3D12 endpoint - no bridge, no synthetic contract, no shaders.
             64-bit only. The add-on the feeder and bridge authors now point
             to for those three APIs.

    NATIVE   the game has DLSS and renders with D3D12.
             Krish's renodx-dlss5 add-on detours the game's own NGX D3D12
             calls. Nothing synthetic, and the game's own quality mode applies.
             The most proven route for D3D12 games that ship DLSS.

    UPSTREAM matiasLombo's neural-upstream add-on. Hooks the same NGX
             EvaluateFeature call but runs the network ITSELF, on the
             render-resolution colour buffer, and hands the result to the
             game's own DLSS as its input. Same-resolution network on a
             smaller image: proportionally cheaper, and the game's quality
             mode still applies. No renodx add-on beside it - two NGX hooks
             fight. Tested on two games by its author.

    OPTI     Dagherbou's OptiScaler fork. Replaces the upscaler and runs the
             model over its output, with a model-resolution dial (25-100%)
             that is the biggest fps lever there is. The author tested RTX 50
             only; with the per-card runtime this tool installs it runs on
             RTX 20/30/40 as well. The game must already use DLSS - or FSR
             2/3 or XeSS: OptiScaler hooks those calls as its INPUT and
             runs DLSS in their place, which is what it is known for.

    BRIDGE   dlss5-bridge: reproduces the DLSS contract on a private D3D12
             session. The route for Vulkan games with DLSS (mirror), and a
             fallback for D3D11.

    FEEDER   DLSS5-Feeder builds a synthetic DLAA contract out of ReShade's
             depth buffer and shader-estimated motion vectors. Works without
             any DLSS in the game, on D3D11/D3D12/Vulkan/OpenGL, and is the
             only way in for 32-bit games (through its host64 helper).

    STANDALONE kibblerz's standalone-dlssnr add-on (DLSS5-Reshade-AIO). Its
             own feed and its own private NGX session - no feeder, no renodx
             add-on. Copies the back buffer at Present, runs the network,
             then DLAA at native resolution or DLSS Super Resolution when the
             game renders below it, frame generation on top, and shows the
             result in a topmost window of its own. Works with or without
             DLSS in the game. Experimental: the window trick is fragile.

    REMIX    the game already has an RTX Remix mod. Remix replaces d3d9.dll
             and path traces the frame in its own runtime (a `.trex` folder),
             and the community forks of that runtime run DLSS-NR inside it,
             after DLSS. No ReShade, no feeder, no add-on - a ReShade proxy
             DLL in the folder crashes a Remix game before it draws.

DirectX 10 is reached by the feeder alone (0.13.1 and later), through a
private D3D11 relay device; nothing else hooks D3D10.
"""
from __future__ import annotations

from dataclasses import dataclass
from pathlib import Path
import time

from . import log, remix

NATIVE, BRIDGE, FEEDER, OPTI, RENODX = "native", "bridge", "feeder", "optiscaler", "renodx"
UPSTREAM = "upstream"
STANDALONE = "standalone"
REMIX = "remix"
ALL_ROUTES = (RENODX, NATIVE, UPSTREAM, OPTI, BRIDGE, FEEDER, STANDALONE,
              REMIX)

# Streamline is NVIDIA's own plugin layer; if a game ships it, it ships DLSS.
# These are never files this tool installs, so they are unambiguous.
STREAMLINE = ("sl.interposer.dll", "sl.dlss.dll", "sl.common.dll",
              "sl.dlss_g.dll", "sl.reflex.dll")
# A game's own runtime, or one renamed by a user to disable it.
OWN_RUNTIME = ("nvngx_dlss.dlsss", "nvngx_dlss.dll.bak", "_nvngx.dll")

# Games do not keep DLSS next to the executable. Unreal ships it under
# Engine/Plugins/Runtime/Nvidia/DLSS/Binaries/ThirdParty/Win64, CryEngine
# (Kingdom Come 2) under Bin/Win64Shared, others under their own names. A
# bounded walk finds them; the folders below never hold DLLs and can be
# hundreds of thousands of files, so they are skipped outright.
DLSS_FILES = ("nvngx_dlss.dll", "sl.interposer.dll", "sl.dlss.dll",
              "nvngx_dlssg.dll", "nvngx_dlssd.dll")
_SKIP_DIRS = {"content", "paks", "saved", "logs", "movies", "sounds", "music",
              "videos", "localization", "shadercache", "derivedcache", "cache",
              "textures", "maps", "levels", "audio", "data", "assets", "mods",
              "screenshots", "steamapps", "redist", "_commonredist", "host64",
              "reshade-shaders"}
# ...but a content folder is skipped for its size, not because a runtime
# cannot be under it: NBA 2K27 keeps Streamline in `data\streamline` (#119),
# and "data" is as generic a folder name as there is. So a skipped folder is
# still entered when one of its own children is named like a place runtimes
# live. One scandir per skipped folder, against a walk that would otherwise
# miss the game's DLSS entirely.
_RUNTIME_HOMES = {"streamline", "dlss", "nvidia", "ngx", "nvngx",
                  "binaries", "plugins", "win64", "x64", "bin"}
_WALK_DEPTH = 9
_WALK_DIRS = 6000
_WALK_SECONDS = 6.0

# A game without DLSS may still ship AMD FSR 2/3 or Intel XeSS. OptiScaler
# hooks those calls as its input ([Inputs] EnableFsr2Inputs / EnableFsr3Inputs
# / EnableFfxInputs / EnableXeSSInputs in its ini) and runs DLSS in their
# place, so such a game can take the OPTI route with no DLSS of its own.
# These are the runtime DLLs the SDKs ship under; a game that statically
# links FSR shows none of them and cannot be told apart from one without.
FSR_FILES = ("ffx_fsr2_api_x64.dll", "ffx_fsr2_api_dx12_x64.dll",
             "ffx_fsr2_api_vk_x64.dll", "ffx_fsr3upscaler_x64.dll",
             "ffx_fsr3_x64.dll", "amd_fidelityfx_dx12.dll",
             "amd_fidelityfx_vk.dll", "ffx_backend_dx12_x64.dll")
XESS_FILES = ("libxess.dll", "libxess_dx11.dll", "libxess_fg.dll")
UPSCALER_FILES = FSR_FILES + XESS_FILES
UPSCALER_NAMES = {"fsr": "AMD FSR 2/3", "xess": "Intel XeSS"}
# OptiScaler's own package carries libxess.dll and amd_fidelityfx_*.dll under
# OptiScaler/ - its bundled upscalers, not the game's. Never evidence.
_UPSCALER_SKIP = ("optiscaler", "licenses")


# folder -> what the bounded walk found, for this run of the program. The
# walk is the expensive part of detection and the game's own runtimes do
# not move while the tool is open, so asking twice is waste. Cleared by
# forget_walk() after an install writes into the folder.
_WALK_CACHE: dict[tuple, list[str]] = {}


def forget_walk(folder=None) -> None:
    """Drop the remembered walk - for one folder, or all of them."""
    if folder is None:
        _WALK_CACHE.clear()
        return
    key = _walk_key(folder)
    # list() copies the keys in one step: a scan or preview worker adding a
    # walk while this runs made the comprehension raise mid-iteration
    for k in [k for k in list(_WALK_CACHE) if k[0] == key]:
        _WALK_CACHE.pop(k, None)


def _walk_key(p) -> str:
    """One spelling of a folder, so forget_walk() clears what walked() set.

    find_dlss_files resolves the path itself, so a trailing separator, an
    8.3 name or a junction would otherwise make two entries for one folder
    and leave a stale answer behind after an install.
    """
    try:
        return str(Path(p).resolve()).lower()
    except OSError:
        return str(p).lower()


def walked(folder: Path, skip_dir: Path | None = None,
           names: tuple[str, ...] = DLSS_FILES) -> list[str]:
    """find_dlss_files, remembered for as long as the program runs.

    `names` is part of the key: the walk stops after six matches, so a
    six-name search and a one-name search are not the same question and
    must not share an answer. Asking for one name can find a runtime that
    the five-name walk spent its budget before reaching.
    """
    key = (_walk_key(folder), _walk_key(skip_dir) if skip_dir else "",
           tuple(names))
    hit = _WALK_CACHE.get(key)
    if hit is None:
        hit = find_dlss_files(folder, skip_dir=skip_dir, names=names)
        try:
            Path(folder).resolve()
            reachable = True
        except OSError:
            # The walk bails out and returns nothing when the folder cannot
            # be resolved - an offline network drive, a sharing violation.
            # Nothing is not an answer worth keeping for the session.
            reachable = False
        if reachable:
            _WALK_CACHE[key] = hit
    return list(hit)          # never the cached list itself


def _holds_runtime(d: Path) -> bool:
    r"""Does this folder have a child named like a place runtimes live?

    The skip list exists so a walk does not crawl a game's whole asset
    tree. It is a size heuristic, and a game that keeps its runtimes under
    one of those names (`data\streamline`, #119) was being read as a game
    with no DLSS at all. Looking one level in costs a single scandir.
    """
    import os as _os
    try:
        with _os.scandir(d) as it:
            for e in it:
                if e.is_dir() and e.name.lower() in _RUNTIME_HOMES:
                    return True
    except OSError:
        pass
    return False


def find_dlss_files(folder: Path, skip_dir: Path | None = None,
                    names: tuple[str, ...] = DLSS_FILES,
                    extra_skip: tuple[str, ...] = ()) -> list[str]:
    """Relative paths of the game's own DLSS files anywhere under `folder`.

    `skip_dir` is the folder this tool installs into: a runtime we put there
    must not count as the game's own (that is what _ours handles there).
    `names` lets the same bounded walk look for other runtimes (FSR, XeSS).
    """
    import os
    out: list[str] = []
    try:
        # Resolve the roots once. Resolving every directory (and skip_dir
        # again for each one) made large engine trees take minutes on Windows.
        folder = Path(folder).resolve()
        skip_dir = Path(skip_dir).resolve() if skip_dir is not None else None
    except OSError:
        return out
    base_depth = len(folder.parts)
    deadline = time.monotonic() + _WALK_SECONDS
    # Skipped folders that were entered only because a runtime-named child
    # is in them: inside one, only those children are followed. Walking all
    # of a big `data` for the sake of its `streamline` cost the budget the
    # rest of the game needed.
    narrow: set[str] = set()
    for seen, (root, dirs, files) in enumerate(os.walk(folder), 1):
        # A depth limit and a match limit do not bound trees with thousands
        # of directories and no runtime DLLs. Check even in the skipped root.
        if seen > _WALK_DIRS or time.monotonic() >= deadline:
            log.write(f"stopped looking for runtime DLLs under {folder} "
                      f"after {seen - 1} folders - search budget reached", "warn")
            break
        rp = Path(root)
        depth = len(rp.parts) - base_depth
        if depth >= _WALK_DEPTH:
            dirs[:] = []
        elif root in narrow:
            dirs[:] = [d for d in dirs if d.lower() in _RUNTIME_HOMES]
        else:
            keep = []
            for d in dirs:
                low = d.lower()
                if low in extra_skip or d.startswith("."):
                    continue
                if low in _SKIP_DIRS:
                    if not _holds_runtime(rp / d):
                        continue
                    narrow.add(os.path.join(root, d))
                keep.append(d)
            dirs[:] = keep
        if rp == skip_dir:
            continue
        for f in files:
            if f.lower() in names:
                try:
                    out.append(str((rp / f).relative_to(folder)))
                except ValueError:
                    out.append(str(rp / f))
        if len(out) >= 6:
            break
    return out


def find_upscaler_files(folder: Path, skip_dir: Path | None = None) -> list[str]:
    """Relative paths of the game's own FSR 2/3 and XeSS runtimes."""
    return find_dlss_files(folder, skip_dir, names=UPSCALER_FILES,
                           extra_skip=_UPSCALER_SKIP)


def upscaler_kind(evidence: list[str]) -> str:
    """"fsr", "xess" or "" for a list of runtime files seen.

    A game shipping both gets "fsr": OptiScaler's FSR 2/3 input hooks are its
    oldest and most exercised, and the ini enables both anyway.
    """
    names = {Path(e).name.lower() for e in evidence}
    if names & set(FSR_FILES):
        return "fsr"
    if names & set(XESS_FILES):
        return "xess"
    return ""


@dataclass
class Support:
    native_dlss: bool = False
    evidence: list[str] = None            # type: ignore[assignment]
    recommended: str = FEEDER
    # Set by _driver_steer when the driver moved the recommendation off a
    # route that loads renodx-dlss5: the route it moved off. The library
    # reads it, so "standalone [experimental]" does not appear there with no
    # reason beside it.
    steered_from: str = ""
    # Set by _shared_steer when the shared results for games like this one
    # moved the recommendation: the route the tool's own rules picked.
    shared_from: str = ""
    reason: str = ""
    options: list[str] = None             # type: ignore[assignment]
    supported: bool = True                # False: no component reaches this API
    why_not: str = ""
    # "fsr" / "xess" / "": the upscaler the game ships when it has no DLSS.
    # OptiScaler can take those calls as its input and run DLSS instead; the
    # GUI passes this into fit() and Options.upscaler.
    upscaler: str = ""
    upscaler_evidence: list[str] = None   # type: ignore[assignment]
    # Where DLSS comes from on the OPTI route, for the reason texts.
    dlss_source: str = ""

    def __post_init__(self) -> None:
        if self.evidence is None:
            self.evidence = []
        if self.options is None:
            self.options = []
        if self.upscaler_evidence is None:
            self.upscaler_evidence = []


# Files that only a DLSS 5 install puts in a game folder. Their presence
# says the nvngx runtimes beside them arrived with that install - whether it
# was this tool, a half-removed one of ours, or a hand-made setup. No game
# ever shipped any of these.
DLSS5_MARKERS = ("dlss5-feed.addon64", "dlss5-feed.addon32", "dlss5-feed.cfg",
                 "dlss5-bridge.addon64", "dlss5-bridge.cfg",
                 "standalone-dlssnr.addon64", "renodx-dlss5.addon64",
                 "nvngx_dlssnr.dll")


def _dlss5_install_here(folder: Path) -> bool:
    """Does this folder plainly carry a DLSS 5 install, record or no record?"""
    from . import installer
    if (folder / installer.MANIFEST).is_file():
        return True
    for name in DLSS5_MARKERS:
        if (folder / name).is_file():
            return True
    return (folder / installer.HOST_DIR).is_dir()


def _ours(folder: Path, name: str) -> bool:
    """Is this file one we installed, rather than the game's own?

    Three signals, in order of how sure each one is: the game's own copy
    kept beside it as a backup, our install record, and - for the case that
    has neither - the unmistakable files of a DLSS 5 install in the same
    folder. That last one is here because a removed or half-finished
    install leaves the runtimes and takes the record with it, and then our
    own nvngx_dlss.dll was read back as proof the GAME shipped DLSS: it sent
    MGS V, a 2015 game with no DLSS at all, down a route meant for games
    that have it. "Own files are never game evidence" - including through
    the door a missing record leaves open.
    """
    from . import installer
    if (folder / (name + installer.BACKUP_SUFFIX)).is_file():
        return True                       # we replaced the game's copy
    man = folder / installer.MANIFEST
    if man.is_file():
        try:
            import json
            data = json.loads(man.read_text(encoding="utf8"))
            return name in data.get("files", [])
        except Exception:
            return False
    # no record: an NGX runtime standing in a folder that carries a DLSS 5
    # install is that install's, not the game's
    return name.lower().startswith("nvngx_") and _dlss5_install_here(folder)


def detect(install_dir: Path, folder: Path, api: str, bitness: int,
           sm: int | None = None, driver: str | None = None,
           shared: dict | None = None, exe=None) -> Support:
    """Work out what the game supports and which path to recommend.

    `sm` is the card's CUDA architecture when known (gpu.detect). On any RTX
    card a D3D12 game with DLSS is steered to OptiScaler, whose
    model-resolution dial is the biggest fps lever there is - and the lever
    matters most on the cards where the pass is heaviest. A card below RTX
    20 gets no such steer; nothing runs there anyway. A D3D12 game with FSR
    or XeSS instead of DLSS is steered only on RTX 50, the cards the author
    tested - one more hook has to land there.

    `shared` is the compatibility list and `exe` the game's executable; with
    both, the shared results for games like this one can move the
    recommendation (_shared_steer). Without them - offline, or a caller that
    does not pass them - the rules here stand as they are.
    """
    s = _detect(install_dir, folder, api, bitness)
    if OPTI in s.options and s.recommended == NATIVE and (sm is None or sm >= 75):
        s.recommended = OPTI
        s.reason = ("This game ships its own DLSS and renders with D3D12: "
                    "OptiScaler runs the model with a model-resolution dial - "
                    "75% costs about half of full size, and the frame itself "
                    "stays full detail. The native route is the simpler, most "
                    "proven alternative."
                    + ("" if (sm or 120) >= 120 else
                       " The author tested RTX 50 only; on your card it runs "
                       "on the community runtime this tool installs."))
    elif (OPTI in s.options and s.upscaler and not s.native_dlss
          and api == "DX12" and sm is not None and sm >= 120):
        # An FSR/XeSS game is a step further from what the author tested
        # (OptiScaler must hook the game's upscaler first), so only the cards
        # they actually used are steered off the feeder; the rest keep the
        # proven route and see OptiScaler as an option.
        s.recommended = OPTI
        s.reason = (f"This game has no DLSS but ships {UPSCALER_NAMES[s.upscaler]} "
                    f"({', '.join(s.upscaler_evidence[:3])}). OptiScaler takes "
                    f"those calls as its input - once FSR/XeSS is switched on "
                    f"in the game's own settings; with no such setting it has "
                    f"nothing to hook - runs DLSS in their place and then "
                    f"neural rendering, with the model-resolution dial. "
                    f"Works in many games, not all - the feeder is the proven "
                    f"fallback.")
    # The shared list is fetched from the web and cached: a malformed copy
    # must not cost the whole game page its routes (gate 2.0.6).
    try:
        _shared_steer(s, api, sm, shared, exe)
    except Exception:
        pass
    _driver_steer(s, driver, folder, install_dir)
    return s


def _shared_steer(s: Support, api: str, sm: int | None, shared: dict | None,
                  exe=None) -> None:
    """Move the recommendation when games like this one say so, clearly.

    `shared` is the compatibility list (community.cached()); None or {} and
    nothing here runs, so an offline machine and every caller that does not
    pass it get the rules above unchanged. The per-game rule comes first
    everywhere else and here too: a game with MIN_REPORTS results of its own
    is steered by those (advice / rank_routes), not by its class. The bar is
    community.class_pick's - MIN_CLASS results on both routes and ranges
    that do not overlap - and the route has to fit this card.
    """
    if not shared or not s.options or s.recommended not in s.options:
        return
    if s.recommended == REMIX:
        return          # a Remix game is never recommended anything else
    from . import community
    if community.reports_for(shared, exe) >= community.MIN_REPORTS:
        return
    klass = community.game_class(api, s.native_dlss, s.upscaler)
    pick, why = community.class_pick(shared, klass, s.options, s.recommended)
    if not pick or not fit(pick, api, s.native_dlss, sm, upscaler=s.upscaler)[0]:
        return
    was = s.recommended
    s.recommended = pick
    s.shared_from = was
    s.reason = (f"{why} Without those results the tool would pick {was}; "
                f"it stays in the list.")


def _driver_steer(s: Support, driver: str | None, folder=None, install_dir=None) -> None:
    """On 616.64 and newer, off the routes that load renodx-dlss5.

    Every route in _RENODX_ROUTES reaches NVIDIA's runtime through the
    renodx-dlss5 add-on, and on these drivers that path faults inside
    D3D12Core.dll on every evaluate in a good number of games. The tool pins
    the add-on to 4.55, which passes on most of them and not on all (#37,
    #92, #107, #173). The standalone add-on does not load renodx-dlss5 at
    all, so it does not take that path.

    The shared results say the same thing so far - standalone has not failed
    yet where a renodx route did - but on a handful of reports, so the
    reason below says that rather than implying a measured rate. Only where
    the route is actually on offer (64-bit D3D11/D3D12): naming a route the
    dropdown does not have is worse than naming none.

    Nothing steers a game that ships its own DLSS: OptiScaler and the native
    route are recommended there, and OptiScaler does not load the add-on.
    """
    from . import gpu, sources
    if not driver or s.recommended not in _RENODX_ROUTES:
        return
    if STANDALONE not in s.options:
        return
    # A game that ships its own DLSS is not steered. The routes recommended
    # there run the game's own DLSS quality mode; standalone ignores it,
    # brings its own feed and presents through its own window, which is a
    # different thing rather than a safer one - and the shared results
    # behind this are all games with no DLSS of their own. (The pre-install
    # driver warning still names it on those routes.)
    if getattr(s, "native_dlss", False):
        return
    if not gpu.driver_at_least(sources.DRIVER_FAULT_MIN, driver):
        return
    # Nor is a game that closes itself the moment ReShade hooks it: those
    # reach DLSS 5 only through DXVK, and the standalone route does not go
    # that way (installer.uses_dxvk leaves it out). Steering MGS V there
    # named the one route that cannot work in that game.
    from . import dxvk as _dxvk
    for _name in _dxvk.NEEDS_DXVK:
        for _where in (folder, install_dir):
            if _where is not None and (_where / _name).is_file():
                return
    was = LABELS.get(s.recommended, s.recommended).split(" - ")[0]
    s.steered_from = s.recommended
    s.recommended = STANDALONE
    s.reason = (f"Driver {driver} is one of the {sources.DRIVER_FAULT_MIN}+ "
                f"builds that fault inside NVIDIA's own NGX runtime on every "
                f"evaluate, and every route that loads the renodx-dlss5 "
                f"add-on - {was} included - goes through the path that "
                f"faults. The tool pins that add-on to "
                f"{sources.DRIVER_FAULT_RENODX_PIN} on this driver, which "
                f"gets most games through and not all. standalone-dlssnr does "
                f"not load it at all, so it never takes that path: it brings "
                f"its own feed and presents through a window of its own. At "
                f"native resolution that needs nothing from you; to run below "
                f"native there is an option for reduced-resolution "
                f"fullscreen and borderless swap chains on its "
                f"'Standalone DLSS-NR + SR' tab in the ReShade overlay "
                f"(reported by the person who got it working, #173). It is the less tested of the two and the reports "
                f"behind this are a handful, so if it does not suit the game, "
                f"{was} is one dropdown away - and rolling the driver back to "
                f"616.56 is the other answer.")


def fit(route: str, api: str, native_dlss: bool, sm: int | None,
        upscaler: str = "") -> tuple[bool, str]:
    """(usable on this machine, short note) for a route the game offers.

    The route list says what the GAME allows; this says what the CARD and
    the route's own rules add to it, so the dropdown can label each entry.

    `upscaler` is Support.upscaler: with no DLSS in the game, OptiScaler is
    usable only when there is an FSR/XeSS call for it to redirect. The
    default keeps the old answers for callers that do not pass it.
    """
    if route == OPTI:
        if not native_dlss and not upscaler:
            return False, "the game must already use DLSS, FSR 2/3 or XeSS"
        note = "model resolution dial: the fps lever"
        if not native_dlss:
            # The evidence for this route is a runtime DLL on disk, and a
            # DLL on disk is not an upscaler the player can switch on: Risk
            # of Rain 2 ships one and exposes no setting, so the route was
            # recommended and then had nothing to hook (#116). The
            # diagnosis said so afterwards; it belongs here, before the
            # install, where the choice is still being made.
            kind = "FSR" if upscaler == "fsr" else "XeSS"
            note = (f"the game's {kind} calls are redirected into DLSS, then "
                    f"neural rendering - so {kind} has to be on in the game's "
                    f"own settings, and if it has no such setting this route "
                    f"has nothing to hook and the feeder route is the one to "
                    f"use; " + note)
        if api == "DX11":
            # Added to, not replaced: a D3D11 game offered this route for its
            # FSR or XeSS still needs to hear that the upscaler has to be on.
            note = ("on D3D11 the upscaler becomes FSR on D3D12"
                    + ("; " + note if not native_dlss else ""))
        if api == "Vulkan":
            # Said before the install now, not only in the diagnosis after
            # it. OptiScaler replaces the game's upscaler through NGX, a
            # Direct3D path; on Vulkan its overlay keeps asking for an
            # upscaler however the game is set (RDR2 twice, #66 and #70).
            note = ("on Vulkan the model has had nothing to attach to in "
                    "the reports received on this route - set the game to "
                    "DirectX 12 if it offers the choice, or use the feeder "
                    "route")
        if sm is not None and sm < 120:
            note += "; author tested RTX 50 only, works here on the community runtime"
        return True, note
    if route == REMIX:
        return True, ("runs inside the Remix runtime, after DLSS - no "
                      "ReShade, no add-on")
    if route == NATIVE:
        return True, "most proven for D3D12 games with DLSS"
    if route == UPSTREAM:
        if not native_dlss:
            return False, "the game must already use DLSS"
        return True, ("runs the network before the game's DLSS, at render "
                      "resolution - cheaper; tested on two games")
    if route == STANDALONE:
        return True, ("own feed: DLAA at native resolution, DLSS SR below it, "
                      "frame generation; experimental - presents through a "
                      "window of its own")
    if route == RENODX:
        return True, "unproven - reported not working in many games; try the others first"
    if route == BRIDGE:
        if api == "Vulkan":
            return True, "mirrors the game's DLSS onto D3D12"
        return True, "mirrors the game's DLSS onto D3D12 - maintained, every release tested on D3D11 and Vulkan"
    if route == FEEDER:
        if api in ("Vulkan", "OpenGL") or not native_dlss:
            return True, "always DLAA, shader motion vectors"
        return True, "always DLAA; ignores the game's DLSS"
    return True, ""


def _detect(install_dir: Path, folder: Path, api: str, bitness: int) -> Support:
    """The route list, then the Remix override.

    A Remix game is a Remix game: the mod has already replaced the renderer,
    DLSS 5 goes inside that runtime, and every ReShade route would be
    injecting a proxy DLL into a process that dies on one. The other routes
    are still listed - people want to see what exists - but none of them is
    ever the recommendation here.
    """
    s = _detect_routes(install_dir, folder, api, bitness)
    trex = remix.find_runtime(install_dir) or remix.find_runtime(folder)
    if trex is None:
        return s
    s.options = [REMIX] + [o for o in s.options if o != REMIX]
    s.recommended = REMIX
    s.supported = True
    s.why_not = ""
    s.reason = (
        "This game has an RTX Remix mod installed (found " + trex.name + "). "
        "Remix path traces the frame in its own runtime, and DLSS 5 runs "
        "INSIDE that runtime, after DLSS - no ReShade, no feeder, no add-on. "
        "The other routes are listed for completeness only: they all install "
        "a ReShade proxy DLL, and a ReShade proxy in a Remix game's folder "
        "crashes it before it draws a frame.")
    return s


DLSSD_NAME = "nvngx_dlssd.dll"
DLSSG_NAME = "nvngx_dlssg.dll"


def _theirs(folder: Path, name: str) -> bool:
    """Did the GAME ship this runtime, rather than this tool writing it?

    Not the same question as `not _ours`. The standalone route ADDS
    nvngx_dlssg.dll to games that never had one, so ours must not be
    reported back as the game's own ("own files are never game evidence").
    Ray reconstruction is only ever SWAPPED, and a swap leaves the game's
    own file beside it as a backup - which is proof the game shipped it,
    even though the file on disk is now ours.
    """
    from . import installer
    if (folder / (name + installer.BACKUP_SUFFIX)).is_file():
        return True
    return not _ours(folder, name)


def _detect_routes(install_dir: Path, folder: Path, api: str,
                   bitness: int) -> Support:
    s = Support()

    for d in {install_dir, folder}:
        for m in STREAMLINE:
            if (d / m).is_file():
                s.native_dlss = True
                s.evidence.append(m)
        for m in OWN_RUNTIME:
            if (d / m).is_file():
                s.native_dlss = True
                s.evidence.append(m)
        # A plain nvngx_dlss.dll counts only when we did not put it there.
        # _theirs: a swap leaves the game's own beside it as a backup, and
        # that proves the game shipped DLSS even though the file on disk is
        # ours now - read as "no DLSS", a swapped game lost its routes and
        # the reinstall that should swap again (gate 1.9.1).
        if (d / "nvngx_dlss.dll").is_file() and _theirs(d, "nvngx_dlss.dll"):
            s.native_dlss = True
            s.evidence.append("nvngx_dlss.dll")
        # The other two runtimes are evidence in their own right, and the
        # GUI reads the ray-reconstruction one out of here to decide whether
        # to offer the swap.
        for m in (DLSSD_NAME, DLSSG_NAME):
            if (d / m).is_file() and _theirs(d, m):
                s.evidence.append(m)
    # The walk runs when the loop above left something to find: the usual
    # "no DLSS beside the executable" case, and one more. An Unreal game
    # puts sl.interposer.dll beside the exe and the runtimes themselves
    # under Engine/Plugins, so native_dlss gets answered without any NGX
    # runtime having been seen - and the GUI reads that answer to decide
    # whether to offer the ray-reconstruction swap.
    #
    # The ray-reconstruction runtime is nested often enough that "found
    # something beside the exe" is not an answer about it: an Unreal game
    # keeps sl.interposer.dll by the executable and the runtimes under
    # Engine/Plugins, and plenty of games keep nvngx_dlss.dll by the
    # executable with nvngx_dlssd.dll deeper in. The walk answers both, and
    # `walked` remembers it, so the budget is paid once per game rather
    # than on every click - which is what #8, #18 and #32 were about.
    answered = s.native_dlss
    seen_rr = any(str(e).lower().endswith(DLSSD_NAME) for e in s.evidence)
    if not answered or not seen_rr:
        # Only the install folder is excluded - anything we wrote lives there.
        for rel in walked(folder, skip_dir=install_dir):
            base = rel.replace("\\", "/").rsplit("/", 1)[-1].lower()
            # `answered` is the state BEFORE this loop. Reading the flag
            # while the loop sets it dropped every hit after the first, so
            # a Streamline game reported less evidence than it had.
            if answered and base not in (DLSSD_NAME, DLSSG_NAME):
                continue
            s.native_dlss = True
            s.evidence.append(rel)
    s.evidence = sorted(set(s.evidence))

    if not s.native_dlss:
        # No DLSS: does the game ship FSR 2/3 or XeSS for OptiScaler to hook?
        # Same two-stage look as DLSS - beside the executable first (the
        # walk skips the install folder, and _ours rules out anything we
        # wrote), then the engine's own folders.
        for d in {install_dir, folder}:
            for m in UPSCALER_FILES:
                if (d / m).is_file() and not _ours(d, m):
                    s.upscaler_evidence.append(m)
        if not s.upscaler_evidence:
            s.upscaler_evidence = find_upscaler_files(folder, skip_dir=install_dir)
        s.upscaler_evidence = sorted(set(s.upscaler_evidence))
        s.upscaler = upscaler_kind(s.upscaler_evidence)
    if s.native_dlss:
        s.dlss_source = "the game's own DLSS"
    elif s.upscaler:
        s.dlss_source = (f"the game's {'FSR 2/3' if s.upscaler == 'fsr' else 'XeSS'} "
                         f"calls, redirected into DLSS by OptiScaler")

    # --- pick a path ----------------------------------------------------
    if api == "DX10":
        # DXGI-based, so ReShade attaches as dxgi.dll like any D3D11 game.
        # Only the feeder reaches D3D10, and only from 0.13.1: it creates a
        # private D3D11 relay device inside the game, because a D3D10 device
        # has no shared NT handles, no fences and no UAVs to hand a frame to
        # the helper with. The add-ons hook D3D9/11/12, the bridge D3D11 and
        # Vulkan - none of them will ever see a D3D10 device.
        s.options = [FEEDER]
        s.recommended = FEEDER
        s.reason = ("Direct3D 10: only the feeder reaches it, from build "
                    "0.13.1 on (it relays the frame through a private D3D11 "
                    "device inside the game). Older feeder builds refuse "
                    "D3D10, so the install checks the build it picked.")
        return s

    if bitness != 64:
        # Every add-on that hooks the game directly is 64-bit only, and a
        # 32-bit process cannot load one. The feeder's host64 helper is the
        # only way in - DX9 games get DXVK in front of it.
        s.options = [FEEDER]
        s.recommended = FEEDER
        s.reason = ("32-bit games can only use the feeder path: every other "
                    "add-on is 64-bit and a 32-bit process cannot load it, so "
                    "the feeder's host64 helper process is the only way in."
                    + (" DirectX 9 is translated to Vulkan by DXVK first."
                       if api == "DX9" else
                       " DirectX 8 is translated to Vulkan by DXVK first "
                       "(its d3d8.dll on its d3d9.dll)."
                       if api == "DX8" else ""))
        return s

    if api == "DX9":
        # 64-bit D3D9 is rare, and only ShortFuse's add-on reaches it: it
        # switches device creation to D3D9Ex and evaluates the presentation
        # backbuffer. The feeder's D3D9 support is the 32-bit DXVK path.
        s.options = [RENODX]
        s.recommended = RENODX
        s.reason = ("64-bit DirectX 9: only the renodx-dlss add-on hooks this "
                    "directly (D3D9Ex, presentation backbuffer). No motion "
                    "vectors on this path, so expect the result to be softer.")
        return s

    if api == "Vulkan":
        # A Vulkan game never loads dxgi.dll; ReShade goes in as a layer, and
        # from there either the bridge mirrors the game's own DLSS contract or
        # the feeder builds one with its own interop.
        if s.native_dlss:
            s.options = [BRIDGE, FEEDER]
            s.recommended = BRIDGE
            s.reason = ("Vulkan game with its own DLSS: the bridge mirrors its "
                        "DLSS contract onto a private D3D12 session, so the "
                        "game's quality mode still applies. The feeder is the "
                        "fallback - always DLAA, with shader motion vectors.")
        else:
            s.options = [FEEDER, BRIDGE]
            s.recommended = FEEDER
            s.reason = ("Vulkan without DLSS: the feeder builds a DLAA contract "
                        "through its own Vulkan interop (no launcher needed "
                        "since 0.5.2). The bridge can instead synthesise one "
                        "from the driver's optical flow - fewer parts, less "
                        "proven.")
        return s

    if api == "OpenGL":
        # Nothing but the feeder reaches OpenGL. It imports a D3D12 device's
        # resources into GL through the memory-object extensions.
        s.options = [FEEDER]
        s.recommended = FEEDER
        s.reason = ("OpenGL is only reachable through the feeder path, which "
                    "shares a D3D12 device's textures into GL. The game must "
                    "actually render on the NVIDIA card.")
        return s

    # 64-bit D3D11 / D3D12 / unknown-but-DXGI from here on.
    if s.native_dlss:
        if api == "DX11":
            s.options = [BRIDGE, OPTI, FEEDER, STANDALONE, RENODX]
            s.recommended = BRIDGE
            s.reason = ("This game has its own DLSS but renders with D3D11, "
                        "which the add-on cannot hook directly. The bridge "
                        "reproduces the contract on a private D3D12 session "
                        "and the game's own quality mode still applies. "
                        "OptiScaler works too but replaces DLSS with FSR on "
                        "D3D11. The renodx-dlss add-on is reported working in "
                        "few games.")
        else:                              # DX12 or unknown -> assume DXGI/D3D12
            s.options = [NATIVE, UPSTREAM, OPTI, BRIDGE, FEEDER, STANDALONE,
                         RENODX]
            s.recommended = NATIVE
            s.reason = ("This game ships its own DLSS and renders with D3D12, "
                        "so the DLSS 5 add-on hooks it directly. No synthetic "
                        "contract, and your in-game DLSS quality setting "
                        "(Quality / Balanced / Performance) still applies. "
                        "OptiScaler adds a model-resolution dial for more fps; "
                        "neural-upstream runs the network before the game's "
                        "DLSS instead, at render resolution, for much less.")
        return s

    # No DLSS of its own, D3D11/D3D12.
    if s.upscaler:
        # OptiScaler's founding trick: hook the game's FSR 2/3 or XeSS calls
        # and run DLSS in their place - then the fork's neural rendering on
        # top. The feeder stays the proven recommendation (detect() steers
        # RTX 50 to OptiScaler); the bridge and renodx-dlss are unchanged.
        # Vulkan is not offered: this tool installs OptiScaler as a DXGI
        # proxy, which a Vulkan game never loads.
        s.options = [FEEDER, OPTI, BRIDGE, STANDALONE, RENODX]
        s.recommended = FEEDER
        s.reason = (f"No DLSS in this game, but it ships "
                    f"{UPSCALER_NAMES[s.upscaler]} "
                    f"({', '.join(s.upscaler_evidence[:3])}). The feeder "
                    f"builds a DLAA contract from ReShade's depth and shader "
                    f"motion vectors - the most proven way. OptiScaler can "
                    f"instead take the game's "
                    f"{'FSR' if s.upscaler == 'fsr' else 'XeSS'} calls as its "
                    f"input and run DLSS in their place, then neural "
                    f"rendering - real upscaling, works in many games, not "
                    f"all. The bridge and the renodx-dlss add-on are the "
                    f"less proven alternatives.")
        return s
    s.options = [FEEDER, BRIDGE, STANDALONE, RENODX]
    s.recommended = FEEDER
    s.reason = ("No DLSS in this game. The feeder builds a DLAA contract from "
                "ReShade's depth and shader motion vectors - the most proven "
                "way. The bridge can instead build one from the driver's "
                "optical flow. standalone-dlssnr brings its own feed with "
                "real DLSS upscaling and frame generation, but presents "
                "through a window of its own - experimental. The renodx-dlss "
                "add-on is the least proven of the four.")
    return s


LABELS = {
    NATIVE: "native - hooks the game's own DLSS (D3D12)",
    UPSTREAM: "neural-upstream - runs before the game's DLSS, cheaper",
    OPTI: "optiscaler - replaces the upscaler, no ReShade",
    BRIDGE: "bridge - the game's DLSS mirrored onto D3D12 (D3D11, Vulkan)",
    FEEDER: "feeder - for games without DLSS (depth + shader motion vectors)",
    STANDALONE: "standalone-dlssnr - own feed, DLSS SR and frame generation",
    RENODX: "renodx-dlss - in-process hook (D3D9/11/12), unproven",
    REMIX: "remix - DLSS 5 inside RTX Remix (path tracing)",
}

# One paragraph per route, shown under the dropdown. What it does, what it
# needs, what it costs - in that order, and nothing a person choosing a
# route does not need.
BLURB = {
    NATIVE: ("Krish's renodx-dlss5 add-on hooks the DLSS calls the game "
             "already makes and adds the neural pass after them. Nothing "
             "synthetic, no shaders; the game's own DLSS quality mode "
             "still applies. 64-bit D3D12 games with DLSS."),
    UPSTREAM: ("matiasLombo's neural-upstream add-on runs the network at "
               "render resolution, BEFORE the game's DLSS upscales - the "
               "same result on a smaller image, so it costs a fraction. "
               "Does the neural pass itself, so no renodx add-on goes in "
               "beside it. Configured from its own tab in ReShade. Its "
               "author tested GTA V Enhanced and Bright Memory Infinite."),
    OPTI: ("No ReShade. OptiScaler takes over the game's upscaler (DLSS, or "
           "FSR 2/3 and XeSS redirected into DLSS) and runs the model over "
           "its output. Its model-resolution dial is the biggest fps lever "
           "there is: the cost falls with the square of it and the frame "
           "keeps full detail. The author tested RTX 50; on RTX 20/30/40 "
           "it runs on the community runtime this tool installs."),
    BRIDGE: ("NIGos' dlss5-bridge mirrors the game's own DLSS contract onto "
             "a private D3D12 session, so a D3D11 or Vulkan game keeps its "
             "DLSS quality mode. Actively maintained; every release is "
             "tested on both APIs."),
    FEEDER: ("jlrouzies-fr's DLSS5-Feeder builds a DLAA contract out of "
             "ReShade's depth buffer and shader-estimated motion vectors, "
             "for games that have no DLSS at all. Always DLAA, never "
             "upscaling. D3D10/11/12, Vulkan, OpenGL, 32-bit (through a "
             "host64 helper) and DirectX 9 (through DXVK)."),
    STANDALONE: ("kibblerz's standalone-dlssnr add-on does the whole "
                 "pipeline itself: its own feed (VORT motion vectors, "
                 "ReShade depth), the network, then DLAA at native "
                 "resolution or real DLSS Super Resolution below it, and "
                 "frame generation on top. Presents through a window of its "
                 "own, which is the fragile part: a resolution or display "
                 "mode change needs a restart. F10 compares. Experimental."),
    RENODX: ("ShortFuse's renodx-dlss add-on hooks D3D9, D3D11 and D3D12 "
             "presentation in-process - no bridge, no shaders. Reported "
             "not working in many games; the only route for 64-bit DirectX "
             "9, otherwise try the recommended one first."),
    REMIX: ("This game has an RTX Remix mod. Remix path traces the frame in "
            "its own runtime (the '.trex' folder) and the neural pass runs "
            "there, after DLSS, so the game's DLSS quality setting applies. "
            "Nothing is injected into the game - no ReShade, no feeder, no "
            "add-on; a ReShade proxy DLL in a Remix folder crashes the game "
            "before it draws. The tool puts nvngx_dlssnr.dll into .trex and "
            "switches the pass on in rtx.conf. If the runtime has no neural "
            "pass (NVIDIA's own has none), 'swap the Remix runtime' puts in "
            "a community build that does - experimental, it can undo fixes "
            "the mod's own runtime carried."),
}

# Per-executable facts the route texts cannot know. Found by people who did
# the install by hand and wrote down why it failed; shown beside the
# conflicts before INSTALL. Keys are lower-case executable names.
QUIRKS: dict[str, str] = {
    "quake3.exe": ("this Quake III executable loads opengl32.dll from "
                   "System32 and never sees ReShade's copy beside it - use "
                   "the ioquake3 build (ioquake3.org), which loads from its "
                   "own folder"),
    "openmw.exe": ("OpenGL: the renodx-dlss5 add-on is pinned to 4.60 here "
                   "(4.70 stalls after four frames on GL) and motion vectors "
                   "come from VORT - LumeniteFX reads none on OpenGL"),
    "forspoken.exe": ("Forspoken checks the signature of the dxgi.dll in its "
                      "folder and closes on ReShade's. Not tried on Forspoken "
                      "yet: on a ReShade route set 'reshade loads as' to "
                      "d3d12.dll; if it refuses that too, the optiscaler route "
                      "with 'loads as' winmm.dll goes through a different file"),
}


# Every route on this list ends up inside the driver's own NGX runtime -
# the add-on asks the driver to evaluate the model. The Remix route does
# not: its runtime carries the neural pass itself.
_NGX_ROUTES = (NATIVE, BRIDGE, FEEDER, OPTI, RENODX, UPSTREAM, STANDALONE)
# ...and these are the ones that get a renodx-dlss5 add-on, so they are the
# only ones the 4.55 pin says anything about. OptiScaler runs the model
# through its own build, the upstream and standalone add-ons through theirs.
_RENODX_ROUTES = (NATIVE, BRIDGE, FEEDER, RENODX)
# The first driver documented to ship the neural-rendering runtime.
# optiscaler.DRIVER_MIN says the same thing for its own route; kept here as
# its own name so this module does not import that one for a constant.
_DRIVER_KNOWN_GOOD = "616.56"


def standalone_fits(api: str, bitness: int | None) -> bool:
    """Is the standalone route one this game is offered at all?

    The same test the route list uses: 64-bit, and D3D11 or D3D12 (or not
    yet known). Anything that suggests standalone asks this first - it was
    being suggested to 32-bit and Vulkan games whose dropdown has no such
    entry. A bitness that is not known is not 64: the route list does not
    offer standalone then either.
    """
    return bitness == 64 and str(api or "").upper() in (
        "DX11", "DX12", "UNKNOWN", "", "?")


def driver_warning(route: str, driver: str | None,
                   offered: list[str] | tuple[str, ...] | None = None) -> str | None:
    """What to say about this driver BEFORE the install, or None.

    616.64 is behind 31 of the first 87 reports - by a distance the largest
    single cause in the tracker, and until now the tool only mentioned it in
    the diagnosis, which is after the person has already lost an evening.
    The renodx-dlss5 build is pinned to 4.55 automatically on this driver,
    but 4.55 faults in some games too (issue #37), so the pin is a
    mitigation and saying otherwise would be a promise the tool cannot keep.
    """
    if not driver or route not in _NGX_ROUTES:
        return None
    from . import gpu, sources
    # The other end of the range, and the one nobody was ever told about: a
    # driver from before DLSS 5 existed cannot create the feature at all.
    # DOOM on 475.14 got "the game has not been started since the install"
    # (#16) - true, and useless. 616.56 is the first driver documented to
    # carry the neural-rendering runtime.
    if gpu.driver_at_least(_DRIVER_KNOWN_GOOD, driver) is False:
        return (f"driver {driver} is older than DLSS 5 itself. The "
                f"neural-rendering runtime first appears in "
                f"{_DRIVER_KNOWN_GOOD}: the install will finish and the model "
                f"will not run. Update the driver before trying anything "
                f"else here.")
    if not gpu.driver_at_least(sources.DRIVER_FAULT_MIN, driver):
        return None

    tail = ("If this game crashes the moment neural rendering comes on, the "
            "driver is the first thing to roll back - 616.56 is the newest "
            "one with no reports of this fault.")
    if route == RENODX:
        # This route installs ShortFuse's renodx-dlss, not renodx-dlss5, and
        # the 4.55 pin lives only in the other branch of the installer - so
        # the mitigation the other routes get does not apply here, and
        # saying it does would be a promise the tool cannot keep.
        return (f"driver {driver}: this route loads ShortFuse's own add-on "
                f"rather than renodx-dlss5, so the {sources.DRIVER_FAULT_RENODX_PIN} "
                f"pin that works around the {sources.DRIVER_FAULT_MIN}+ fault "
                f"elsewhere does not apply here, and nothing is known either "
                f"way about this add-on on those drivers. " + tail)
    if route in _RENODX_ROUTES:
        # The fault lives in renodx-dlss5's path through the driver. The
        # shared results showed a game fail three times on the feeder route
        # on 616.92 and then work on standalone, reported by the same person - so
        # for a game without DLSS there is a way round that needs no
        # rollback, and it was never mentioned here. Said as the mechanism,
        # not as a count: the counts belong to the compatibility note,
        # which is per game and keeps itself current.
        # Only where standalone is actually on offer: 32-bit, DX9, Vulkan
        # and OpenGL games are not given it, and naming a route that is not
        # in the dropdown is worse than naming none.
        # Every route here loads renodx-dlss5, so every one of them has the
        # same way round; the evidence is one game on the feeder route.
        sa = (STANDALONE in offered) if offered is not None else route == FEEDER
        other = ("The standalone route does not load renodx-dlss5 at all, and "
                 "is worth trying before the driver. If that crashes too the "
                 "moment neural rendering comes on, roll the driver back - "
                 "616.56 is the newest one with no reports of this fault."
                 if sa else tail)
        return (f"driver {driver}: the 4.6/4.7 renodx-dlss5 builds fault "
                f"inside the driver's NGX runtime on "
                f"{sources.DRIVER_FAULT_MIN} and newer, on every evaluate - "
                f"the tool pins {sources.DRIVER_FAULT_RENODX_PIN} here "
                f"instead. That is a way round it, not a fix: some games "
                f"fault on {sources.DRIVER_FAULT_RENODX_PIN} too. " + other)
    if route == STANDALONE:
        # This route was being warned off with the renodx fault's record,
        # which is not its own: it does not load that add-on, and the one
        # game in the shared results that failed on feeder on 616.92 worked
        # here. It still runs NVIDIA's runtime on this driver, so the tail
        # stays - but the scare does not.
        return (f"driver {driver}: the fault known on "
                f"{sources.DRIVER_FAULT_MIN} and newer is reached through the "
                f"renodx-dlss5 add-on, which this route does not load - so on "
                f"these drivers this is the route to try when a route that "
                f"loads it (native, bridge, feeder) faults. It still runs NVIDIA's runtime: if the game crashes "
                f"the moment neural rendering comes on, roll the driver back "
                f"to 616.56.")
    return (f"driver {driver}: {sources.DRIVER_FAULT_MIN} and newer fault "
            f"inside the driver's own NGX runtime in a good number of games, "
            f"and {sources.DRIVER_FAULT_MIN} itself is named in more "
            f"reports here than any other single cause. This route does not "
            f"install the renodx add-on, "
            f"so the pin that works around it elsewhere does not apply. "
            + tail)


def quirks(exe, api: str = "") -> tuple[str, ...]:
    """The QUIRKS lines for this executable and API, if any."""
    name = getattr(exe, "name", exe or "") or ""
    out: list[str] = []
    if api == "OpenGL":
        out.append("OpenGL: motion vectors come from VORT (LumeniteFX reads "
                   "none here) and the renodx-dlss5 add-on is pinned to 4.60 - "
                   "both applied at install, whatever the dropdowns say")
    hit = QUIRKS.get(str(name).lower())
    if hit and hit not in out:
        out.append(hit)
    return tuple(out)


# What must NOT sit in the folder or run beside each route, and what breaks
# when it does. Short and honest: these are the conflicts people actually
# hit, not a legal disclaimer. ReShade loads every .addon64 it finds, and
# two things hooking the same NGX calls means flicker or nothing at all.
# Each line is (kind, text):
#   "folder" - something that must not be in the game folder. The tool looks
#              before it says it, and names the file when it is there; a
#              clean folder is not told about it every time.
#   "ingame" - a setting the person has to change in the game. Nothing can
#              check that, so it is always on screen.
#   "note"   - what the route does. Read once, then in the way.
CONFLICTS: dict[str, tuple[tuple[str, str], ...]] = {
    NATIVE: (
        ("folder",
             "not with OptiScaler or a frame-gen unlocker in the same folder "
             "- two NGX hooks: flicker, greyed-out frame-gen or nothing"),
        ("ingame",
             "NVIDIA Smooth Motion off for this game"),
    ),
    UPSTREAM: (
        ("folder",
             "not with the renodx-dlss5 add-on, OptiScaler or another NGX "
             "hook in the folder - installing removes ours, name theirs"),
        ("ingame",
             "with DLSS Frame Generation set its cadence to Quality in the "
             "overlay, or expect stutter"),
        ("note",
             "does not upscale - the game's own DLSS still does"),
    ),
    OPTI: (
        ("folder",
             "no ReShade at all on this route; other RenoDX add-ons will not load"),
        ("note",
             "the game must already use DLSS, FSR 2/3 or XeSS"),
        ("folder",
             "not with a frame-gen unlocker or dlss-enabler in the folder"),
        ("ingame",
             "with 'frame generation' on (D3D12): the game's own frame "
             "generation must be OFF"),
    ),
    BRIDGE: (
        ("folder",
             "not with the feeder or renodx-dlss add-on in the same folder "
             "- both build a contract and the game dies before its swap chain"),
        ("ingame",
             "NVIDIA Smooth Motion off for this game"),
        ("note",
             "an older dlss5-dx11-bridge.addon64 is removed - the two conflict"),
    ),
    FEEDER: (
        ("note",
             "always DLAA; the game's own DLSS is ignored"),
        ("ingame",
             "NVIDIA Smooth Motion off"),
        ("folder",
             "not with the bridge or renodx-dlss add-on in the same folder"),
    ),
    RENODX: (
        ("folder",
             "not with the renodx-dlss5 add-on, the feeder or the bridge in "
             "the folder - they all hook NGX"),
        ("note",
             "reported not working in many games; nothing to tune if it does "
             "nothing, switch route"),
    ),
    STANDALONE: (
        ("ingame",
             "the game's own DLSS, frame generation and anti-aliasing "
             "must be OFF - it brings its own"),
        ("note",
             "presents through its own topmost window; resolution or "
             "display-mode changes need a restart"),
        ("folder",
             "not with the renodx add-on, OptiScaler or neural-upstream "
             "in the folder"),
    ),
    REMIX: (
        ("folder",
             "no ReShade, no feeder, no add-ons in the folder - a ReShade "
             "proxy DLL crashes a Remix game before it draws"),
        ("note",
             "the neural pass runs inside the Remix runtime, after DLSS, so "
             "the game's own DLSS/RR settings still apply"),
        ("ingame",
             "toggle it in the Remix menu: Alt+X -> Developer Settings Menu "
             "-> Post-Processing"),
    ),
}


```

## Source file: core/dlssupdate.py

{
  "path": "core/dlssupdate.py",
  "blob_sha": "05b88de81b25046a6c00a0b140489ad83843ca71",
  "sha256": "b5714c0a7eb6e29bf4bb4a19e2efa911e5faa925c3c47bde0b888d827adacfcd",
  "lines": 955,
  "bytes": 39421,
  "selection_score": 15,
  "keyword_line_counts": {
    "skin": 1,
    "dlss": 65,
    "rig": 28
  },
  "url": "https://github.com/Kizzuwatnaa/DLSS5-Autopilot/blob/5897489e6469418498d6522c933674518c930ca8/core/dlssupdate.py",
  "function_signature_candidates": [
    {
      "line": 121,
      "text": "def name(self) -> str:"
    },
    {
      "line": 124,
      "text": "def to_json(self) -> dict:"
    },
    {
      "line": 139,
      "text": "def ok(self) -> bool:"
    },
    {
      "line": 148,
      "text": "def __init__(self, text: str, kind: str = \"\"):"
    },
    {
      "line": 155,
      "text": "def _ver(s: str) -> tuple:"
    },
    {
      "line": 159,
      "text": "def _number(label: str) -> str:"
    },
    {
      "line": 166,
      "text": "def _key(p) -> str:"
    },
    {
      "line": 173,
      "text": "def _rel(folder: Path, p: Path) -> str:"
    },
    {
      "line": 188,
      "text": "def _is_x64(p: Path) -> bool:"
    },
    {
      "line": 195,
      "text": "def _install_backup(p: Path) -> Path | None:"
    },
    {
      "line": 204,
      "text": "def _ours(p: Path) -> Path:"
    },
    {
      "line": 208,
      "text": "def _same_build(a: Path, b: Path) -> bool:"
    },
    {
      "line": 214,
      "text": "def _set_aside(p: Path, suffix: str, runtime: Path | None = None) -> Path:"
    },
    {
      "line": 229,
      "text": "def _size(p: Path) -> int:"
    },
    {
      "line": 238,
      "text": "def record_path(g) -> Path:"
    },
    {
      "line": 242,
      "text": "def load_record(g) -> list[dict]:"
    },
    {
      "line": 294,
      "text": "def _save_record(g, rows: list[dict]) -> None:"
    },
    {
      "line": 306,
      "text": "def _find(rows: list[dict], rel: str) -> dict | None:"
    },
    {
      "line": 310,
      "text": "def _written_here(p: Path, r: dict) -> bool:"
    },
    {
      "line": 324,
      "text": "def _matches(p: Path, version: str, r: dict) -> bool:"
    },
    {
      "line": 335,
      "text": "def _manifest_files(g, extra_dirs) -> set[str]:"
    },
    {
      "line": 368,
      "text": "def _walk_hits(g, folder: Path) -> list[str]:"
    },
    {
      "line": 388,
      "text": "def _candidates(g, fresh: bool) -> list[Path]:"
    },
    {
      "line": 419,
      "text": "def scan(g, fresh: bool = False) -> list[Entry]:"
    },
    {
      "line": 483,
      "text": "def newest(catalog: dict | None = None, refresh: bool = False) -> dict[str, dict]:"
    },
    {
      "line": 519,
      "text": "def outdated(e: Entry, new: dict | None) -> bool:"
    },
    {
      "line": 532,
      "text": "def running(g, all_procs=None) -> str:"
    },
    {
      "line": 547,
      "text": "def anticheat_name(g) -> str:"
    },
    {
      "line": 551,
      "text": "def anticheat_info(g) -> tuple[str, str]:"
    },
    {
      "line": 563,
      "text": "def _probe_write(folder: Path) -> str:"
    },
    {
      "line": 575,
      "text": "def games_hint(folder: Path) -> str:"
    },
    {
      "line": 583,
      "text": "def _guards(g, allow_anticheat: bool, rep: Report) -> bool:"
    },
    {
      "line": 605,
      "text": "def settle(g) -> list[str]:"
    },
    {
      "line": 663,
      "text": "def _copy_whole(src: Path, dst: Path) -> None:"
    },
    {
      "line": 677,
      "text": "def _replace_one(g, e: Entry, build: dict, progress) -> str:"
    },
    {
      "line": 778,
      "text": "def update(g, families=None, build: str | None = None, allow_anticheat: bool = False,"
    },
    {
      "line": 836,
      "text": "def restore(g, families=None) -> Report:"
    },
    {
      "line": 934,
      "text": "def cache_path() -> Path:"
    },
    {
      "line": 939,
      "text": "def load_cache() -> dict:"
    },
    {
      "line": 949,
      "text": "def save_cache(data: dict) -> None:"
    }
  ]
}

```text

r"""Keeping a game's own NVIDIA DLSS runtimes current, with or without DLSS 5.

A game ships super resolution (nvngx_dlss.dll), frame generation
(nvngx_dlssg.dll) and ray reconstruction (nvngx_dlssd.dll) where its engine
loads them, and keeps whatever build it shipped with for years. This module
finds them, compares them with NVIDIA's own newest build, replaces them, and
puts the game's own back on request.

The record
----------
`dlss5-dlss-update.json` in the game's root folder lists every file this
module replaced: its path relative to that folder, the family, the game's
own version, and the version, size and SHA-256 of what was written. The
game's own file is kept beside it as `<name>.dlss5-dlss-original`.

That suffix is deliberately NOT the DLSS 5 installer's `.dlss5-autopilot-
backup`. The installer's uninstall restores every backup with its own suffix
it finds in the install folder, and reads one beside a runtime as "this
install swapped it". Sharing it would make uninstalling DLSS 5 silently undo
a DLSS update, and would make the next DLSS 5 install treat the game's
runtime as its own. With separate suffixes the two writers stack:

    game's own  ->  DLSS update  ->  DLSS 5 install swap
                    (.dlss5-dlss-original)  (.dlss5-autopilot-backup)

and each one takes back only its own layer.

Invariants
----------
1. The game's own file exists on disk at all times: as the runtime itself,
   as our `.dlss5-dlss-original`, as the DLSS 5 install's backup, or set
   aside under a suffix of ours. No operation here deletes a runtime that is
   not a build this module wrote itself, or a backup that is not an exact
   copy of the file on disk; a runtime in the way is renamed
   `.dlss5-dlss-displaced-<time>`.
2. A backup is made from the file on disk when there is none, and never
   overwritten. Updating a build this module wrote keeps the first backup,
   and with that backup gone makes none: the file on disk is then byte for
   byte what this module wrote (the record's hash of it), not the game's
   own, and filing it as the original would let restore put it over a
   genuine file later. A backup found with no record (the record was lost)
   is adopted as the original.
3. A file the DLSS 5 install swapped (its backup sits beside it) is not
   updated here: the install owns that layer, and its uninstall would put
   back what was under it. The page says to change it in the game's
   settings.
4. A DLSS 5 install over a file updated here backs up OUR build as "what was
   there", and its uninstall restores it - so uninstalling DLSS 5 leaves the
   DLSS update in place. Restoring here while DLSS 5 is installed on top
   hands the game's own file to the install's backup slot - only when that
   backup is the build this module wrote; otherwise nothing moves and the
   page says to uninstall DLSS 5 first.
5. A file that is no longer what was written (a launcher verified its files,
   the game updated itself, another tool) is left alone. Its backup and
   record entry stay - another tool may put our build back, and only the
   backup holds the game's own - until the file on disk is an exact copy of
   the backup (the launcher put the game's own back): then the backup is a
   duplicate, and it and the entry go.
6. Restore with the backup missing leaves the updated file in place, drops
   the entry and says so. A runtime that disappeared while its backup is
   there is put back by restore.
7. Everything read from the record is untrusted: an entry must resolve
   inside the game (or its install folder) and name one of the three
   runtimes, and the backup path is derived from it, never read. Whether a
   backup is needed is decided from the disk, not from the record; the
   record only withholds one when the file's own bytes hash to what it says
   was written. Its version and size are not enough: a record naming the
   game's own build would otherwise send that file away unkept.
8. The download is a 64-bit Windows DLL of a sane size before anything is
   touched, the runtime is re-read right before it is replaced (another
   writer may have been at it), and it is replaced whole (written beside,
   then renamed over), never half.
9. One writer at a time: the window runs these jobs under the same busy flag
   installs, autopilot and uninstall wait for.
"""
from __future__ import annotations

import json
import os
import shutil
import tempfile
import time
from dataclasses import dataclass, field
from pathlib import Path

from . import anticheat, dlss, installer, log, net, pe, prefs, sources, watch

FAMILIES = (("dlss", "nvngx_dlss.dll", "super resolution"),
            ("dlssg", "nvngx_dlssg.dll", "frame generation"),
            ("dlssd", "nvngx_dlssd.dll", "ray reconstruction"))
FILE_OF = {f: n for f, n, _ in FAMILIES}
FAMILY_OF = {n: f for f, n, _ in FAMILIES}
LABEL = {f: lab for f, _, lab in FAMILIES}
ORDER = {f: i for i, (f, _, _) in enumerate(FAMILIES)}

RECORD = "dlss5-dlss-update.json"
ORIGINAL_SUFFIX = ".dlss5-dlss-original"
DISPLACED_SUFFIX = ".dlss5-dlss-displaced"      # a runtime restore or update found in the way
PART_SUFFIX = ".dlss5-dlss-part"
CACHE_NAME = "dlss-scan.json"

ORIGINAL, UPDATED, INSTALL, MISSING = "original", "updated", "install", "missing"

# Refusal kinds, so the page can draw each one as its own state.
R_32BIT, R_RUNNING, R_ANTICHEAT, R_READONLY, R_OFFLINE, R_NOTHING = (
    "32bit", "running", "anticheat", "readonly", "offline", "nothing")


@dataclass
class Entry:
    path: Path
    rel: str
    family: str
    version: str
    state: str = ORIGINAL
    original: str = ""            # the game's own version, once swapped
    label: str = ""               # the build this module wrote there
    backup: Path | None = None    # our copy of the game's own, when present

    @property
    def name(self) -> str:
        return FILE_OF[self.family]

    def to_json(self) -> dict:
        return {"rel": self.rel, "family": self.family, "version": self.version,
                "state": self.state, "original": self.original, "label": self.label,
                "backup": self.backup is not None}


@dataclass
class Report:
    done: list[str] = field(default_factory=list)
    skipped: list[str] = field(default_factory=list)
    notes: list[str] = field(default_factory=list)
    error: str = ""
    refused: str = ""             # one of the R_ kinds when nothing was tried

    @property
    def ok(self) -> bool:
        return not self.error and not self.refused


# "Nothing was changed" is true of one runtime; after another went through it is not
_NOTHING = __import__("re").compile(r"[;,.]?\s*[Nn]othing was changed[.;]?")


class _Stop(Exception):
    def __init__(self, text: str, kind: str = ""):
        super().__init__(text)
        self.kind = kind


# ---------------------------------------------------------------- helpers

def _ver(s: str) -> tuple:
    return installer._ver(s)


def _number(label: str) -> str:
    """"310.9.1" from "310.9.1 (NVIDIA SDK)"."""
    import re
    m = re.match(r"\s*v?(\d+(?:\.\d+)*)", label or "")
    return m.group(1) if m else ""


def _key(p) -> str:
    try:
        return os.path.normcase(os.path.realpath(str(p)))
    except OSError:
        return os.path.normcase(os.path.abspath(str(p)))


def _rel(folder: Path, p: Path) -> str:
    """One spelling per file: resolved, relative to the resolved game folder.

    A junction or an 8.3 name would otherwise give the same runtime two
    record keys, or an absolute one the record then refuses.
    """
    try:
        return Path(os.path.realpath(str(p))).relative_to(os.path.realpath(str(folder))).as_posix()
    except (ValueError, OSError):
        try:
            return Path(p).relative_to(folder).as_posix()
        except ValueError:
            return str(p).replace("\\", "/")


def _is_x64(p: Path) -> bool:
    try:
        return pe.exe_bitness(p) == 64
    except (pe.PEError, OSError):
        return False


def _install_backup(p: Path) -> Path | None:
    """The DLSS 5 install's backup beside this runtime, when it swapped it."""
    for s in (installer.BACKUP_SUFFIX,) + tuple(installer.LEGACY_BACKUP_SUFFIXES):
        b = p.with_name(p.name + s)
        if b.is_file():
            return b
    return None


def _ours(p: Path) -> Path:
    return p.with_name(p.name + ORIGINAL_SUFFIX)


def _same_build(a: Path, b: Path) -> bool:
    """Two files of one build (version and size) - a copy, not a swap."""
    va = pe.file_version(a)
    return bool(va) and _ver(va) == _ver(pe.file_version(b)) and _size(a) == _size(b) >= 0


def _set_aside(p: Path, suffix: str, runtime: Path | None = None) -> Path:
    """Rename a file out of the way, never over another file.

    Named after the runtime it belongs to (`nvngx_dlss.dll<suffix>-<time>`),
    so a backup set aside does not end up with two suffixes.
    """
    stamp = int(time.time())
    for n in range(100):
        target = p.with_name(f"{(runtime or p).name}{suffix}-{stamp}" + (f"-{n}" if n else ""))
        if not target.exists():
            os.replace(p, target)
            return target
    raise OSError(f"no free name to set {p.name} aside")


def _size(p: Path) -> int:
    try:
        return p.stat().st_size
    except OSError:
        return -1


# ---------------------------------------------------------------- the record

def record_path(g) -> Path:
    return Path(g.folder) / RECORD


def load_record(g) -> list[dict]:
    """The record's entries that are safe to act on (invariant 7)."""
    p = record_path(g)
    try:
        data = json.loads(p.read_text(encoding="utf8"))
    except (OSError, ValueError):
        return []
    rows = data.get("files") if isinstance(data, dict) else None
    out: list[dict] = []
    seen: set[str] = set()
    for e in rows if isinstance(rows, list) else []:
        if not isinstance(e, dict):
            continue
        rel, fam = e.get("path"), e.get("family")
        if not isinstance(rel, str) or fam not in FILE_OF:
            continue
        target = None
        for base, absolute in ((Path(g.folder), False), (Path(g.install_dir), True)):
            try:
                target = net.inside(base, rel, absolute_ok=absolute)
                break
            except (net.OutsideError, OSError):
                continue
        if target is None:
            log.write(f"dlss update record: ignored an entry outside the game folder: {rel}", "warn")
            continue
        if target.name.lower() != FILE_OF[fam]:
            log.write(f"dlss update record: ignored an entry that is not a {FILE_OF[fam]}: {rel}", "warn")
            continue
        import posixpath
        rel = posixpath.normpath(rel.replace("\\", "/"))
        # one entry per FILE: an absolute and a relative spelling of one
        # runtime would otherwise be two entries, one of them always stale
        ident = _key(target)
        if ident in seen:
            continue
        seen.add(ident)
        if Path(rel).is_absolute():
            rel = _rel(Path(g.folder), target)
        try:
            size = int(e.get("size") or 0)
        except (TypeError, ValueError):
            size = 0
        digest = str(e.get("sha256") or "").lower()
        out.append({"path": rel, "family": fam, "original": str(e.get("original") or ""),
                    "written": str(e.get("written") or ""), "size": size,
                    "label": str(e.get("label") or ""), "at": e.get("at") or 0,
                    "sha256": digest if len(digest) == 64
                    and all(ch in "0123456789abcdef" for ch in digest) else ""})
    return out


def _save_record(g, rows: list[dict]) -> None:
    p = record_path(g)
    if not rows:
        try:
            p.unlink(missing_ok=True)
        except OSError:
            pass
        return
    installer._write_atomic(p, json.dumps({"tool": "dlss5-autopilot", "kind": "dlss-update",
                                           "version": 1, "files": rows}, indent=2))


def _find(rows: list[dict], rel: str) -> dict | None:
    return next((r for r in rows if r["path"].lower() == rel.lower()), None)


def _written_here(p: Path, r: dict) -> bool:
    """Is the file on disk, byte for byte, what this module wrote there?

    The record's hash of the written file, taken right after writing it. A
    record without one (older, or hand-made) proves nothing."""
    want = r.get("sha256") or ""
    if len(want) != 64:
        return False
    try:
        return net.sha256(p) == want
    except OSError:
        return False


def _matches(p: Path, version: str, r: dict) -> bool:
    """Is the file on disk still what this module wrote?"""
    if not r.get("written") or not version:
        return False
    if _ver(version) != _ver(r["written"]):
        return False
    return not r.get("size") or _size(p) == r["size"]


# ---------------------------------------------------------------- finding

def _manifest_files(g, extra_dirs) -> set[str]:
    """Every path a DLSS 5 install recorded as written, as comparable keys."""
    folder = Path(g.folder)
    roots = {Path(g.install_dir), folder} | set(extra_dirs)
    try:
        for d in prefs.installs():
            dp = Path(d)
            if dp == folder or folder in dp.parents:
                roots.add(dp)
    except Exception:
        pass
    out: set[str] = set()
    for r in roots:
        for name in (installer.MANIFEST,) + tuple(installer.LEGACY_MANIFESTS):
            m = r / name
            try:
                if not m.is_file():
                    continue
                data = json.loads(m.read_text(encoding="utf8", errors="replace"))
            except (OSError, ValueError):
                continue
            files = (data.get("files") or data.get("dosyalar") or []) if isinstance(data, dict) else []
            for f in files if isinstance(files, list) else []:
                if isinstance(f, str) and f.lower().endswith(tuple(FAMILY_OF)):
                    out.add(_key(r / f))
    return out


# find_dlss_files stops after this many matches; a remembered walk with fewer
# ran to its end (or to the budget, which a narrower walk would hit too).
_WALK_MATCH_STOP = 6


def _walk_hits(g, folder: Path) -> list[str]:
    """The three runtimes under the folder, from detection's walk when it has one.

    The library's detection walks every game with all of DLSS_FILES and the
    install folder skipped (dlss.detect); asking again with three names is
    another key in the walk cache, so the page's read 5 s after start walked
    every game a second time. A walk that ran to its end holds every one of
    the three; the install folder's own files are added by the caller.
    """
    names = tuple(FAMILY_OF)
    try:
        wide = dlss._WALK_CACHE.get((dlss._walk_key(folder), dlss._walk_key(g.install_dir),
                                     tuple(dlss.DLSS_FILES)))
    except Exception:
        wide = None
    if wide is not None and len(wide) < _WALK_MATCH_STOP:
        return [h for h in wide if h.replace("\\", "/").rsplit("/", 1)[-1].lower() in names]
    return dlss.walked(folder, names=names)


def _candidates(g, fresh: bool) -> list[Path]:
    folder = Path(g.folder)
    if fresh:
        dlss.forget_walk(folder)
    found: list[Path] = []
    try:
        # The same bounded, remembered walk detection uses (scan budgets),
        # asked only for the three runtimes this page is about.
        for h in _walk_hits(g, folder):
            found.append(folder / h)
    except Exception:
        log.exception(f"looking for DLSS files in {folder}")
    for base in {Path(g.install_dir), folder}:
        for n in FAMILY_OF:
            if (base / n).is_file():
                found.append(base / n)
    # A file this module updated is looked at even when the walk ran out of
    # budget before reaching it: the record knows where it is.
    for r in load_record(g):
        p = folder / r["path"]
        if p.is_file():
            found.append(p)
    seen, out = set(), []
    for p in found:
        k = _key(p)
        if k not in seen and p.name.lower() in FAMILY_OF:
            seen.add(k)
            out.append(p)
    return out


def scan(g, fresh: bool = False) -> list[Entry]:
    """The DLSS runtimes this game loads, with what each one is. Read-only.

    64-bit games only, and 64-bit files only. Not listed: anything under our
    host64 helper, and any runtime a DLSS 5 install wrote where the game had
    none (it is in the install's record with no backup beside it). Listed as
    MISSING: a runtime this module replaced that is gone while the game's own
    is still beside where it was.
    """
    if getattr(g, "bitness", None) != 64 or not getattr(g, "folder", None):
        return []
    folder = Path(g.folder)
    cands = _candidates(g, fresh)
    rows = load_record(g)
    written = _manifest_files(g, {p.parent for p in cands}) if cands else set()
    out: list[Entry] = []
    for p in cands:
        rel = _rel(folder, p)
        if any(part.lower() == installer.HOST_DIR for part in Path(rel).parts):
            continue
        if not _is_x64(p):
            continue
        ibak = _install_backup(p)
        if ibak is None and _key(p) in written:
            continue                      # our own file, never the game's
        e = Entry(path=p, rel=rel, family=FAMILY_OF[p.name.lower()], version=pe.file_version(p))
        r = _find(rows, rel)
        bak = _ours(p)
        has_bak = bak.is_file()
        if ibak is not None:
            # the DLSS 5 install's layer is on top - even when it wrote the
            # very build this module wrote before it
            e.state = INSTALL
            if has_bak:
                e.backup = bak
                e.original = r["original"] if r is not None else pe.file_version(bak)
        elif r is not None and _matches(p, e.version, r):
            e.state = UPDATED
            e.original, e.label = r["original"], r["label"]
            e.backup = bak if has_bak else None
        elif has_bak and not _same_build(p, bak):
            # Our copy of the game's own, beside a runtime that is not what the
            # record says was written: the record is gone, or something changed
            # the file since. Restorable; restore sets the file there aside.
            e.state = UPDATED
            e.backup = bak
            e.original = pe.file_version(bak)
        # else the game's own - also a backup of the very same build, which is
        # what a crash between the backup and the swap leaves
        out.append(e)
    for r in rows:
        p = folder / r["path"]
        if not p.exists() and _ours(p).is_file():
            out.append(Entry(path=p, rel=r["path"], family=r["family"], version="", state=MISSING,
                             original=r["original"], label=r["label"], backup=_ours(p)))
    out.sort(key=lambda e: (ORDER[e.family], e.rel.lower()))
    return out


# ---------------------------------------------------------------- newest

_NEWEST: dict[str, dict] = {}


def newest(catalog: dict | None = None, refresh: bool = False) -> dict[str, dict]:
    """{family: catalog entry + "version"} - NVIDIA's own newest per family.

    Network, so called from a worker. Remembered for the run once it has an
    answer; an empty one is asked again next time. `refresh` asks again
    anyway - the page's "check again", where a window left open for days
    would otherwise never see a build NVIDIA published meanwhile.
    """
    global _NEWEST
    remember = catalog is None
    if refresh and catalog is None:
        # sources remembers the tag for the run as well; forgetting only ours
        # would read the same old answer back. What we had stays until a new
        # answer replaces it, so an update right after a failed check still
        # has a build to take.
        sources._NVIDIA_CACHE = None
    if catalog is None:
        if _NEWEST and not refresh:
            return dict(_NEWEST)
        try:
            catalog = sources.nvidia_dlss()
        except Exception as e:
            log.write(f"reading NVIDIA's DLSS builds failed: {e}", "warn")
            catalog = {}
    out: dict[str, dict] = {}
    for fam in FILE_OF:
        entries = [x for x in (catalog or {}).get(fam) or [] if isinstance(x, dict) and x.get("url")]
        if not entries:
            continue
        best = max(entries, key=lambda x: _ver(_number(str(x.get("label", "")))))
        out[fam] = dict(best, version=_number(str(best.get("label", ""))))
    if remember and out:
        _NEWEST = out
    return out


def outdated(e: Entry, new: dict | None) -> bool:
    """Is there a newer build than the one on disk? Never for an install's swap."""
    if not new or e.state in (INSTALL, MISSING) or not e.version:
        return False
    if e.label and e.label == new.get("label"):
        # NVIDIA versions each runtime on its own: a file written from this
        # build is current even when its stamp reads lower than the SDK tag.
        return False
    return _ver(e.version) < _ver(new.get("version", ""))


# ---------------------------------------------------------------- guards

def running(g, all_procs=None) -> str:
    """The name of a process running out of the game's folder, or "".

    `all_procs` is one process snapshot shared by a scan of many games:
    taking a fresh one per game reads every process's image path each time.
    """
    try:
        ps = watch.from_folder(Path(g.folder), all_procs)
        if not ps and Path(g.install_dir) != Path(g.folder):
            ps = watch.from_folder(Path(g.install_dir), all_procs)
    except Exception:
        return ""
    return ps[0].name if ps else ""


def anticheat_name(g) -> str:
    return anticheat_info(g)[0]


def anticheat_info(g) -> tuple[str, str]:
    """(the anti-cheat's name, "found: <files>") or ("", "").

    The files go into the page's question too: every other anti-cheat
    warning names what it rests on, so the person can check it is real."""
    try:
        f = anticheat.detect(Path(g.install_dir), Path(g.folder))
        return (f.summary, f.found) if f.present else ("", "")
    except Exception:
        return "", ""


def _probe_write(folder: Path) -> str:
    """"" when a file can be created here, else where Windows refused."""
    try:
        with tempfile.NamedTemporaryFile(prefix=".dlss5-dlss-write-test-", dir=folder):
            pass
        return ""
    except OSError as e:
        why = e.strerror or type(e).__name__
        hint = games_hint(folder)
        return f"Windows refused to write into {folder} ({why}).{hint}"


def games_hint(folder: Path) -> str:
    from . import games
    if games.is_locked_store_path(folder):
        return " " + games.XBOX_HINT
    return (" Close the game and its launcher; if it is still refused, the folder needs "
            "administrator rights.")


def _guards(g, allow_anticheat: bool, rep: Report) -> bool:
    if getattr(g, "bitness", None) != 64:
        rep.refused = R_32BIT
        rep.error = "Only 64-bit games are offered: NVIDIA publishes these runtimes for 64-bit Windows."
        return False
    name = running(g)
    if name:
        rep.refused = R_RUNNING
        rep.error = (f"{name} is running from this game's folder. Close it first - Windows does not let "
                     f"anything replace a file a running program has open.")
        return False
    ac = anticheat_name(g)
    if ac and not allow_anticheat:
        rep.refused = R_ANTICHEAT
        rep.error = (f"{ac} is installed with this game. An anti-cheat can treat a changed DLSS file as "
                     f"tampering; for anything played online, keep the game's own.")
        return False
    return True


# ---------------------------------------------------------------- acting

def settle(g) -> list[str]:
    """Drop record entries whose file is no longer what was written (invariant 5).

    Never deletes a backup: it cannot tell a launcher putting the game's own
    back from another tool writing a newer build, and only the first leaves a
    copy of the game's file anywhere. Writes only when there is a record, so
    for a game this module never touched it is a read.
    """
    rows = load_record(g)
    if not rows:
        return []
    folder = Path(g.folder)
    keep, said = [], []
    for r in rows:
        p = folder / r["path"]
        name = p.name
        bak = _ours(p)
        if not p.exists():
            if bak.is_file():
                keep.append(r)            # restore puts the game's own back (invariant 6)
            else:
                said.append(f"{name} is no longer at {r['path']}")
            continue
        v = pe.file_version(p)
        if _matches(p, v, r) or _install_backup(p) is not None:
            keep.append(r)
            continue
        if bak.is_file():
            old = pe.file_version(bak)
            if _same_build(p, bak):
                # The launcher put exactly the game's own back: the backup is a
                # second copy of the file on disk, and only then is it removed.
                try:
                    bak.unlink()
                except OSError:
                    pass
                said.append(f"{name}: the game's own {v} is back (a launcher verified its files)")
            else:
                # Anything else - a newer build from the game, or another tool
                # that may put our build back later - keeps the game's own
                # where scan and restore find it, and keeps the entry: a later
                # settle still sees the launcher's exact copy, and our build
                # coming back still reads as ours.
                keep.append(r)
                said.append(f"{name}: changed to {v or 'another build'} since the update; "
                            f"restore original still puts the game's own {old} back")
        else:
            said.append(f"{name}: changed to {v or 'another build'} by something else - left as it is")
    if len(keep) != len(rows):
        try:
            _save_record(g, keep)
        except OSError as e:
            said.append(f"the record could not be written ({e})")
    for s_ in said:
        log.write(f"dlss update, {getattr(g, 'name', folder)}: {s_}")
    return said


def _copy_whole(src: Path, dst: Path) -> None:
    part = dst.with_name(dst.name + PART_SUFFIX)
    try:
        shutil.copyfile(src, part)
        if _size(part) != _size(src):
            raise OSError(f"the copy of {src.name} came out short")
        os.replace(part, dst)
    finally:
        try:
            part.unlink(missing_ok=True)
        except OSError:
            pass


def _replace_one(g, e: Entry, build: dict, progress) -> str:
    folder = e.path.parent
    why = _probe_write(folder)
    if why:
        raise _Stop(why, R_READONLY)
    try:
        got = net.download(build["url"], f"{e.family}-{build['label']}.dll", progress=progress)
    except Exception as ex:
        raise _Stop(f"{e.name} could not be downloaded: {str(ex).splitlines()[0] if str(ex) else ex}. "
                    f"Nothing was changed.", R_OFFLINE)
    ok, why = installer._is_win64_dll(Path(got))
    want = int(build.get("size") or 0)
    if ok and want and _size(Path(got)) != want:
        ok, why = False, f"the download is {_size(Path(got))} bytes where {want} were published"
    if not ok:
        try:
            Path(got).unlink()
        except OSError:
            pass
        raise _Stop(f"{e.name}: {why}. Nothing was changed.")
    # Invariant 8: re-read it now. The download takes a while, and a launcher
    # or an install may have written this very file in the meantime.
    now = pe.file_version(e.path) if e.path.is_file() else ""
    if not e.path.is_file() or _ver(now) != _ver(e.version) or _install_backup(e.path) is not None:
        raise _Stop(f"{e.name} changed while the update was downloading ({e.version or '?'} -> "
                    f"{now or 'gone'}). Nothing was changed; check again.")
    rows = load_record(g)
    r = _find(rows, e.rel)
    bak = _ours(e.path)
    made, aside = False, None
    if r is not None and _matches(e.path, e.version, r) and bak.is_file():
        pass                             # our own build, the game's own beside it: that backup stands
    elif r is not None and _matches(e.path, e.version, r) and _written_here(e.path, r):
        # Our own build with the game's copy gone (deleted by hand, cleaned
        # by a tool). A "backup" made now would be OUR build filed as the
        # game's own: restore would later put it over a genuine file a
        # launcher verified, and the page would label it the game's. The
        # record keeps the game's own version; the page says its backup is
        # gone, as it did before this update. Version and size alone do not
        # prove it (a record naming the game's own build is not trusted,
        # invariant 7): the bytes must be the ones this module wrote.
        log.write(f"dlss update: the backup of the game's own {e.name} is gone; "
                  f"{e.version} there is the build this tool wrote, so it is not backed up", "warn")
    elif not bak.is_file():
        # Invariant 2/7: no copy of what is there yet, and it is not the
        # build the record says was written - so it is the game's.
        try:
            _copy_whole(e.path, bak)
        except OSError as ex:
            raise _Stop(f"the game's own {e.name} could not be backed up ({ex}); nothing was changed.",
                        R_READONLY)
        made = True
        if r is None:
            r = {"path": e.rel, "family": e.family}
            rows.append(r)
        r["original"] = e.version
    else:
        # A backup of the game's own is there, and the file is not a build
        # this module wrote (the record is gone, or the file changed since).
        if not _same_build(e.path, bak):
            try:
                aside = _set_aside(e.path, DISPLACED_SUFFIX)
            except OSError as ex:
                raise _Stop(f"Windows refused to move {e.path} aside ({ex.strerror or ex}). Nothing was changed.",
                            R_READONLY)
        if r is None:
            r = {"path": e.rel, "family": e.family}
            rows.append(r)
        r["original"] = pe.file_version(bak)
    try:
        _copy_whole(Path(got), e.path)
    except OSError as ex:
        if aside is not None:
            try:
                os.replace(aside, e.path)
            except OSError:
                pass
        if made:
            try:
                bak.unlink()             # the game's own is still in place
            except OSError:
                pass
        raise _Stop(f"Windows refused to replace {e.path} ({ex.strerror or ex}). Nothing was changed. "
                    f"Close the game and its launcher; a read-only file needs its attribute cleared.",
                    R_READONLY)
    was = e.version
    try:
        digest = net.sha256(e.path)
    except OSError:
        digest = ""
    r.update(written=pe.file_version(e.path) or _number(build["label"]), size=_size(e.path),
             label=build["label"], at=round(time.time()), sha256=digest)
    try:
        _save_record(g, rows)
    except OSError as ex:
        # The backup beside it still makes it restorable (scan adopts it).
        log.write(f"dlss update record not written: {ex}", "warn")
    line = f"{LABEL[e.family]}  {was or '?'} -> {r['written']}"
    return line + (f" (the file that was there is kept as {aside.name})" if aside is not None else "")


def update(g, families=None, build: str | None = None, allow_anticheat: bool = False,
           progress=None, catalog: dict | None = None) -> Report:
    """Replace the game's DLSS runtimes with NVIDIA's newest (or `build`).

    `families` limits it ("dlss", "dlssg", "dlssd"); None is every outdated
    one. `progress(done, total)` follows the download.
    """
    rep = Report()
    if not _guards(g, allow_anticheat, rep):
        return rep
    rep.notes += settle(g)
    entries = [e for e in scan(g) if families is None or e.family in families]
    if not entries:
        rep.refused = R_NOTHING
        rep.error = "No 64-bit DLSS runtime of this game was found to update."
        return rep
    news = newest(catalog)
    if not news:
        rep.refused = R_OFFLINE
        rep.error = ("NVIDIA's build list could not be read (raw.githubusercontent.com). "
                     "Check the connection and try again.")
        return rep
    for e in entries:
        if e.state == INSTALL:
            rep.skipped.append(f"{LABEL[e.family]}: set by the DLSS 5 install - change it in the game's settings")
            continue
        if e.state == MISSING:
            rep.skipped.append(f"{LABEL[e.family]}: the file is gone - restore original puts the game's own back")
            continue
        new = news.get(e.family)
        if new is None:
            rep.skipped.append(f"{LABEL[e.family]}: NVIDIA publishes no build of it")
            continue
        if build:
            pool = (catalog if catalog is not None else sources.nvidia_dlss()).get(e.family) or []
            wanted = next((x for x in pool if x.get("label") == build or x.get("tag") == build), None)
            if wanted is None:
                rep.skipped.append(f"{LABEL[e.family]}: no build {build}")
                continue
            new = dict(wanted, version=_number(wanted["label"]))
        elif not outdated(e, new):
            rep.skipped.append(f"{LABEL[e.family]}: {e.version} is current")
            continue
        try:
            rep.done.append(_replace_one(g, e, new, progress))
        except _Stop as ex:
            # after a runtime that did go through, "nothing was changed" is untrue
            rep.error = _NOTHING.sub("", str(ex)).rstrip(" ;,") if rep.done else str(ex)
            if ex.kind == R_READONLY and not rep.done:
                rep.refused = R_READONLY
            break
    for line in rep.done:
        log.write(f"dlss update, {getattr(g, 'name', '')}: {line}")
    if rep.error:
        log.write(f"dlss update, {getattr(g, 'name', '')}: {rep.error}", "warn")
    return rep


def restore(g, families=None) -> Report:
    """Put the game's own runtimes back and drop the record.

    Driven by the record and the backups on disk, not by scan(): a runtime
    that is gone, unreadable or no longer 64-bit still has its original put
    back.
    """
    rep = Report()
    rep.notes += settle(g)
    rows = load_record(g)
    folder = Path(g.folder)
    paths: dict[str, Path] = {}
    for r in rows:
        paths.setdefault(_key(folder / r["path"]), folder / r["path"])
    for p in _candidates(g, False):
        if _ours(p).is_file():
            paths.setdefault(_key(p), p)
    targets = sorted((p for p in paths.values() if p.name.lower() in FAMILY_OF
                      and (families is None or FAMILY_OF[p.name.lower()] in families)),
                     key=lambda p: (ORDER[FAMILY_OF[p.name.lower()]], str(p).lower()))
    if not targets:
        return rep
    name = running(g)
    if name:
        rep.refused = R_RUNNING
        rep.error = f"{name} is running from this game's folder. Close it first."
        return rep
    for p in targets:
        rel = _rel(folder, p)
        r = _find(rows, rel)
        fam = FAMILY_OF[p.name.lower()]
        label = LABEL[fam]
        bak = _ours(p)
        ibak = _install_backup(p)
        drop = True
        if ibak is not None:
            if bak.is_file() and r is not None and _matches(ibak, pe.file_version(ibak), r):
                # Invariant 4: the install's backup is the build we wrote; hand
                # it the game's own, so the install's uninstall restores that.
                try:
                    os.replace(bak, ibak)
                except OSError as ex:
                    rep.error = f"Windows refused to move {bak} ({ex.strerror or ex}). Nothing was changed."
                    break
                rep.notes.append(f"{label}: DLSS 5 is installed over it - the game's own "
                                 f"{r['original'] or 'file'} comes back when DLSS 5 is uninstalled")
            elif bak.is_file():
                drop = False
                rep.notes.append(f"{label}: DLSS 5 was installed over a file that changed since the update - "
                                 f"uninstall DLSS 5 first, then restore original")
            else:
                rep.notes.append(f"{label}: set by the DLSS 5 install; nothing of ours is left under it")
        elif not bak.is_file():
            if p.exists():
                v = pe.file_version(p)
                rep.notes.append(f"{label}: the backup of the game's own {p.name} is gone - the {v or 'file there'} "
                                 f"stays. Verify the game's files in its launcher to get the original back.")
            else:
                rep.notes.append(f"{label}: {p.name} and its backup are both gone. Verify the game's files in "
                                 f"its launcher to get it back.")
        else:
            was = pe.file_version(p) if p.exists() else ""
            aside = None
            try:
                if p.exists() and not (r is not None and _matches(p, was, r)) and not _same_build(p, bak):
                    aside = _set_aside(p, DISPLACED_SUFFIX)     # invariant 1: not ours, not deleted
                _copy_whole(bak, p)
            except OSError as ex:
                if aside is not None and not p.exists():
                    try:
                        os.replace(aside, p)
                    except OSError:
                        pass
                rep.error = (f"Windows refused to replace {p} ({ex.strerror or ex}). Nothing was changed. "
                             f"Close the game and its launcher.")
                break
            try:
                bak.unlink()
            except OSError:
                pass
            rep.done.append(f"{label}  {was or 'missing'} -> {pe.file_version(p) or '?'}")
            if aside is not None:
                rep.notes.append(f"{label}: the {was or 'file'} that was there is kept as {aside.name}")
        if drop and r is not None:
            rows.remove(r)
        try:
            _save_record(g, rows)
        except OSError as ex:
            rep.notes.append(f"the record could not be written ({ex})")
    for line in rep.done + rep.notes:
        log.write(f"dlss restore, {getattr(g, 'name', '')}: {line}")
    if rep.error:
        log.write(f"dlss restore, {getattr(g, 'name', '')}: {rep.error}", "warn")
    return rep


# ---------------------------------------------------------------- the page's cache

def cache_path() -> Path:
    # Read at call time: tests point prefs.FILE at a temporary folder.
    return prefs.FILE.parent / CACHE_NAME


def load_cache() -> dict:
    try:
        data = json.loads(cache_path().read_text(encoding="utf8"))
    except (OSError, ValueError):
        return {}
    if not isinstance(data, dict) or not isinstance(data.get("games"), dict):
        return {}
    return data


def save_cache(data: dict) -> None:
    try:
        p = cache_path()
        p.parent.mkdir(parents=True, exist_ok=True)
        installer._write_atomic(p, json.dumps(data, indent=1))
    except OSError as e:
        log.write(f"the dlss scan could not be saved: {e}", "warn")


```

## Source file: dlss5_autopilot.py

{
  "path": "dlss5_autopilot.py",
  "blob_sha": "ef6faf339459aa9fdb6a55189b6a61db9066be4a",
  "sha256": "cd57ececbe633bdbf4d5a9d20537dd3edfadc3a9da581e199134da6ba35439b4",
  "lines": 344,
  "bytes": 16442,
  "selection_score": 15,
  "keyword_line_counts": {
    "dlss": 28
  },
  "url": "https://github.com/Kizzuwatnaa/DLSS5-Autopilot/blob/5897489e6469418498d6522c933674518c930ca8/dlss5_autopilot.py",
  "function_signature_candidates": [
    {
      "line": 60,
      "text": "def _console() -> None:"
    },
    {
      "line": 81,
      "text": "def cli(target: Path, remove: bool, check: bool, route: str = \"\","
    },
    {
      "line": 257,
      "text": "def main() -> int:"
    }
  ]
}

```text

r"""DLSS 5 Autopilot - entry point.

GUI:            dlss5-autopilot.exe
Command line:   dlss5-autopilot.exe "D:\Games\Game" [--check | --remove]
                                                    [--route native|upstream|optiscaler|renodx|bridge|feeder|standalone|remix]
                                                    [--dxvk | --no-dxvk] [--remix-swap] [--vr]
                                                    [--no-gpu-pref]
                                                    [--opti-build y4my4my4m|wilsjo2|wilsjo2-mfg|janblade]
                                                    [--own-fg "D:\Downloads\version.dll" | sm86 | enabler]
                dlss5-autopilot.exe --video ["D:\DLSS5 Player"]  the video player

--dxvk runs a D3D11 game on Vulkan through DXVK, with ReShade as a Vulkan
layer instead of a DLL inside the game. Games known to need it (MGS V) get
it by default; --no-dxvk turns that off.

--remix-swap applies to --route remix only: when the game's RTX Remix mod
ships a runtime with no DLSS 5 neural pass, replace it with a community
build that has one. Experimental - it also replaces whatever game-specific
fixes the mod's own runtime carried.

--vr registers ReShade's OpenXR layer as well, for a game that draws
through OpenXR. The desktop window is only a mirror, so a proxy DLL on its
own changes nothing in the headset; OpenVR/SteamVR titles are not reached
either way. The registration is per user rather than per game, and the
last VR uninstall removes it. Untried with a headset here.

--no-gpu-pref leaves Windows' graphics setting alone. By default, on a
machine with an NVIDIA card and another GPU (a laptop), the game - and the
32-bit feeder's helper - are set to High performance so they draw on the
NVIDIA card; a choice already made there is kept, --remove takes it out.

--opti-build applies to --route optiscaler only and picks a fork other
than Dagherbou's: y4my4my4m (multi-pass, multi-frame generation),
wilsjo2 (the neural pass before the upscaler - installed with that
placement switched on, which is off in the fork's own default), or
wilsjo2-mfg (the same, from that fork's RTX 40 multi-frame generation
package; refused on a card known not to be an RTX 40), or janblade (a fork
of wilsjo2's, the same placement). None of them has been run here.

--own-fg applies to --route optiscaler on a D3D12 game only: frame
generation files you downloaded yourself - dlssg_for_sm86 (its version.dll,
with dlssg_sm86.ini beside it, or one of its alternatives\ proxies with the
ini copied beside it) or DLSS Enabler (its version.dll). Give the path of
the dll the first time; the tool keeps a copy, and after that its name is
enough: sm86, sm86-winmm, sm86-dxgi, sm86-d3d12, sm86-dbghelp or enabler.
Experimental, and out again on --remove.
"""
from __future__ import annotations

import sys
from pathlib import Path

if __package__ in (None, ""):
    sys.path.insert(0, str(Path(__file__).resolve().parent))

from core import (community, dlss, games, gpu, installer,  # noqa: E402
                  optiscaler, prefs, reshade_ini, update)


def _console() -> None:
    """The exe is built without a console window, so attach to the calling
    terminal when run from one - otherwise CLI output goes nowhere."""
    try:
        import ctypes
        ATTACH_PARENT = -1
        if ctypes.windll.kernel32.AttachConsole(ATTACH_PARENT):
            sys.stdout = open("CONOUT$", "w", encoding="utf-8", errors="replace",
                              buffering=1)
            sys.stderr = open("CONOUT$", "w", encoding="utf-8", errors="replace",
                              buffering=1)
            return
    except Exception:
        pass
    try:
        sys.stdout.reconfigure(encoding="utf-8", errors="replace")
        sys.stderr.reconfigure(encoding="utf-8", errors="replace")
    except Exception:
        pass


def cli(target: Path, remove: bool, check: bool, route: str = "",
        dxvk: bool | None = None, game=None, remix_swap: bool = False,
        vr: bool = False, opti_build: str = "", gpu_pref: bool = True,
        own_fg: str = "") -> int:
    g = game or games.manual(target)
    if not g.exe:
        print(f"error: no executable found in {target}", file=sys.stderr)
        return 1
    need = installer.wants_dxvk(g)
    use_dxvk = bool(need) if dxvk is None else dxvk
    card, sm = gpu.detect()
    sup = dlss.detect(g.install_dir, g.folder, g.api, g.bitness or 0, sm,
                      driver=gpu.driver_version(), shared=community.cached(),
                      exe=g.exe)
    level, why_rel = installer.reliability(g, sup.recommended)
    print(f"game    : {g.name}")
    if card:
        drv = gpu.driver_version()
        print(f"gpu     : {card} ({gpu.label(sm)})" + (f"  driver {drv}" if drv else ""))
        # ...the warning itself is printed after --route has had its say,
        # a few lines below: it depends on which route is actually used.
    else:
        vendor = gpu.other_vendor()
        if vendor == "AMD":
            print("gpu     : no NVIDIA card detected")
            for line in gpu.AMD_ANSWER.splitlines():
                print("          " + line.strip())
        else:
            print("gpu     : no NVIDIA card detected - dlss5 will not run")
    print(f"exe     : {g.exe}")
    print(f"arch    : {g.bit_label}   API: {g.api} ({g.api_why})")
    if route:
        if route not in sup.options:
            print(f"error: route '{route}' is not available for this game "
                  f"(options: {', '.join(sup.options)})", file=sys.stderr)
            return 1
        sup.recommended = route
        # An explicit --route is the answer; the reason and the outlook
        # belong to the recommendation it just overrode - printing either
        # would describe a route this run is not taking.
        sup.reason = ""
        level, why_rel = installer.reliability(g, sup.recommended)
    if card:
        # The same warning the install page shows, for the route that will
        # actually be installed. A command-line install on a driver that
        # cannot run any of this used to proceed in silence.
        warn = dlss.driver_warning(sup.recommended, gpu.driver_version(),
                                   offered=sup.options)
        if warn:
            print(f"driver  : {warn}")
        said = community.driver_note(community.cached(), gpu.driver_version() or "",
                                     sup.recommended)
        if said:
            print(f"shared  : {said}")
    print(f"route   : {dlss.LABELS[sup.recommended]}")
    for o in sup.options:
        usable, note = dlss.fit(o, g.api, sup.native_dlss, sm,
                                upscaler=sup.upscaler)
        print(f"          {'*' if o == sup.recommended else ' '} {o:<11}"
              f"{'' if usable else 'NOT FOR THIS PC - '}{note}")
    if sup.native_dlss:
        print(f"          this game ships its own DLSS "
              f"({', '.join(sup.evidence[:3])})")
    elif sup.upscaler:
        print(f"          this game ships {dlss.UPSCALER_NAMES[sup.upscaler]} "
              f"and no DLSS ({', '.join(sup.upscaler_evidence[:3])})")
    if sup.reason:
        print(f"          {sup.reason}")
    print(f"outlook : {level} - {why_rel}")
    if use_dxvk:
        print(f"dxvk    : yes - {need + ' closes itself when ReShade hooks it; ' if need else ''}"
              f"the game will render on Vulkan and ReShade loads as a Vulkan "
              f"layer (--no-dxvk to turn this off)")

    if own_fg and sup.recommended != installer.OPTI:
        print("note    : --own-fg applies to --route optiscaler only; ignored here")
        own_fg = ""
    elif own_fg and g.api != "DX12":
        # the window shows the row on D3D12 games only: DLSS frame
        # generation is D3D12 (gate 2.0.7)
        print(f"note    : --own-fg applies to D3D12 games only ({g.api} here); ignored")
        own_fg = ""
    elif own_fg:
        from core import ownfg as _ofg
        print(f"fg files: {_ofg.label(own_fg)} - {', '.join(_ofg.dests(own_fg))} "
              f"({_ofg.UNTRIED})")

    ok, why = installer.check_supported(g)
    if check:
        # The RTX 40 MFG package on another card is refused by install()
        # before a file is written; --check said "ready" over it (gate
        # 2.0.5). The same check, the same words.
        refused = (optiscaler.card_refusal(opti_build, sm)
                   if ok and sup.recommended == installer.OPTI else "")
        if ok and not refused and own_fg:
            refused = installer._own_fg_refusal(g.install_dir, installer.Options(
                path=sup.recommended, own_fg=own_fg))
        print(f"installed: {'yes' if g.installed else 'no'}")
        print(f"status   : {refused or ('ready' if ok else why)}")
        # The one mode whose whole job is "look, write nothing" was the one
        # that never said the executable is a launcher: the warning reached
        # the command line only in install()'s report, after the files were
        # already written.
        launcher = installer.launcher_warning(g)
        if launcher:
            print(f"warning  : {launcher}")
        local, _ = prefs.find_renodx()
        print(f"renodx   : {local.name if local else 'will download from the mirror'}")
        if ok:
            popt = installer.Options(path=sup.recommended,
                                     native_dlss=sup.native_dlss, dxvk=use_dxvk, vr=vr, opti_build=opti_build,
                                     upscaler=sup.upscaler,
                                     remix_swap=remix_swap, gpu_pref=gpu_pref, own_fg=own_fg)
            print(f"plan     : {' -> '.join(installer.plan(g, popt))}")
            # ...and what the MFG package needs from the game, which only
            # the finished install used to say.
            for w in installer._mfg_warnings(g, popt, g.install_dir):
                print(f"warning  : {w}")
            if popt.opti_build == optiscaler.PRESR_MFG and popt.fg and g.api == "DX12":
                print(f"warning  : {installer.MFG_FSR_FG}")
            gp = installer._gpu_pref_line(g.install_dir, g, popt)
            if gp:
                print(f"outside  : {gp} (--no-gpu-pref to skip)")
        return 1 if refused else 0
    if not ok:
        print(f"error: {why}", file=sys.stderr)
        return 1

    if remove:
        installer.uninstall(g, on_log=print)
        return 0

    try:
        rep = installer.install(
            g, installer.Options(path=sup.recommended, native_dlss=sup.native_dlss,
                                 dxvk=use_dxvk, vr=vr, opti_build=opti_build, upscaler=sup.upscaler,
                                 remix_swap=remix_swap, gpu_pref=gpu_pref, own_fg=own_fg),
            on_log=print,
            on_prog=lambda p, m: print(f"\r  {p:3d}%  {m:<60}", end="", flush=True))
    except installer.InstallError as e:
        print(f"\nerror: {e}", file=sys.stderr)
        return 1
    if installer.record_lost(rep):
        print(f"\n\nInstalled, but no record was saved - {len(rep.written)} files written.")
    else:
        print(f"\n\nDone - {len(rep.written)} files written.")
    for w in rep.warnings:
        print(f"  ! {w}")
    # The remix route has no ReShade overlay at all - its settings live in
    # the Remix menu, so the usual advice would send people to a key that
    # does nothing there.
    if sup.recommended == dlss.REMIX or route == dlss.REMIX:
        print("In game: press Alt+X, then 'Developer Settings Menu', then the "
              "Post-Processing tab: 'Enable Neural Uplift (DLSS-NR)'.")
    elif sup.recommended == dlss.FEEDER and g.bitness == 32:
        # See the matching note in gui.py: the DLSS 5 page is in the game's
        # own overlay and drives the 64-bit helper; the helper's separate
        # window must NOT be alt-tabbed to while playing.
        print(f"In game: press {reshade_ini.overlay_key_name()}, then turn on "
              f"neural rendering in the "
              "DLSS 5 page (F6 toggles it) - it drives the 64-bit helper for "
              "you. Play BORDERLESS or true fullscreen at your display's own "
              "resolution: in a bordered window the game presents a few "
              "pixels short and the result never lands on screen. Do NOT "
              "alt-tab to the helper's window while playing. Turn the game's "
              "own MSAA/SSAA off.")
    else:
        # The route's own key, not ReShade's: OptiScaler opens on Insert.
        _dflt = (optiscaler.OVERLAY_KEY if sup.recommended == dlss.OPTI
                 else "Home")
        print(f"In game: press {reshade_ini.overlay_key_name(_dflt)}, then "
              f"enable neural rendering in the DLSS 5 panel. Turn the game's "
              f"own MSAA/SSAA off.")
    return 0


def main() -> int:
    args = list(sys.argv[1:])
    route = ""
    if "--route" in args and args.index("--route") + 1 < len(args):
        route = args[args.index("--route") + 1]
        args.remove(route)
    opti_build = ""
    if "--opti-build" in args and args.index("--opti-build") + 1 < len(args):
        _console()      # the exe has no console of its own; errors below need one
        opti_build = args[args.index("--opti-build") + 1]
        args.remove(opti_build)
        from core import optiscaler as _opti
        if opti_build not in _opti.BUILDS:
            print(f"error: --opti-build takes one of: "
                  f"{', '.join(k for k in _opti.BUILDS if k)}", file=sys.stderr)
            return 2
        if route and route != "optiscaler":
            print("note: --opti-build applies to --route optiscaler only; "
                  "ignored here", file=sys.stderr)
    own_fg = ""
    if "--own-fg" in args:
        _console()
        if args.index("--own-fg") + 1 >= len(args):
            print("error: --own-fg takes the path of the .dll you downloaded, or a name "
                  "you used before", file=sys.stderr)
            return 2
        picked = args[args.index("--own-fg") + 1]
        args.remove(picked)
        # a set's name in any case (SM86); anything else is a path
        if picked.lower() in __import__("core.ownfg", fromlist=["RECIPES"]).RECIPES:
            picked = picked.lower()
        from core import ownfg as _ofg
        try:
            if picked in _ofg.RECIPES:
                if not _ofg.stored(picked):
                    raise _ofg.OwnFgError(
                        f"no copy of {_ofg.label(picked).split(' (')[0]} is stored yet - give the "
                        f"path of the .dll you downloaded")
                own_fg = picked
            else:
                own_fg, files = _ofg.identify([Path(picked)])
                _ofg.store(own_fg, files)
        except (_ofg.OwnFgError, OSError) as e:
            print(f"error: --own-fg: {e}", file=sys.stderr)
            return 2
    positional = [a for a in args if not a.startswith("-")]
    if "--video" in args:
        _console()
        from core import video
        folder = Path(positional[0]) if positional else video.default_dir()
        print(f"video player -> {folder}")
        try:
            g = video.prepare(folder, on_log=print,
                              on_prog=lambda p, m: print(f"\r  {p:3d}%  {m:<60}",
                                                         end="", flush=True))
        except Exception as e:
            print(f"\nerror: {e}", file=sys.stderr)
            return 1
        print()
        return cli(g.exe, remove="--remove" in args, check="--check" in args,
                   route=route or "feeder", dxvk=False, game=g)
    if positional:
        _console()
        return cli(Path(positional[0]),
                   remove="--remove" in args,
                   check="--check" in args,
                   route=route,
                   vr="--vr" in args,
                   opti_build=opti_build,
                   own_fg=own_fg,
                   dxvk=(True if "--dxvk" in args
                         else False if "--no-dxvk" in args else None),
                   remix_swap="--remix-swap" in args,
                   gpu_pref="--no-gpu-pref" not in args)
    if "--help" in args or "-h" in args:
        _console()
        print(__doc__)
        return 0
    if "--version" in args:
        _console()
        print(update.VERSION)
        return 0
    from core.ui import app
    return app.run()


if __name__ == "__main__":
    sys.exit(main())


```

## Source file: core/ui/ctl_dlss.py

{
  "path": "core/ui/ctl_dlss.py",
  "blob_sha": "dc06c2efbdf56de6ad6d91867359389051d49f25",
  "sha256": "30a5713e6a61e13ed08a1b28c0f3c662b331485bdc51ba39303ac0703c095d89",
  "lines": 546,
  "bytes": 25113,
  "selection_score": 15,
  "keyword_line_counts": {
    "sss": 4,
    "dlss": 186,
    "rig": 3
  },
  "url": "https://github.com/Kizzuwatnaa/DLSS5-Autopilot/blob/5897489e6469418498d6522c933674518c930ca8/core/ui/ctl_dlss.py",
  "function_signature_candidates": [
    {
      "line": 23,
      "text": "def _first_line(text: str) -> str:"
    },
    {
      "line": 29,
      "text": "def _dlss_init(self) -> None:"
    },
    {
      "line": 46,
      "text": "def dlss_state(self) -> dict:"
    },
    {
      "line": 51,
      "text": "def _dlss_games(self) -> list:"
    },
    {
      "line": 62,
      "text": "def _dlss_stamp(g) -> float:"
    },
    {
      "line": 77,
      "text": "def _dlss_entry(folder: str, d: dict) -> du.Entry:"
    },
    {
      "line": 84,
      "text": "def dlss_newest(self) -> dict:"
    },
    {
      "line": 88,
      "text": "def dlss_row(self, g) -> dict | None:"
    },
    {
      "line": 102,
      "text": "def _dlss_rank(r) -> tuple:"
    },
    {
      "line": 108,
      "text": "def dlss_rows(self) -> list[dict]:"
    },
    {
      "line": 117,
      "text": "def dlss_todo(self) -> list[dict]:"
    },
    {
      "line": 121,
      "text": "def dlss_line(self, g) -> str:"
    },
    {
      "line": 137,
      "text": "def dlss_line_quiet(self, g) -> bool:"
    },
    {
      "line": 142,
      "text": "def dlss_background(self) -> None:"
    },
    {
      "line": 155,
      "text": "def dlss_recheck_running(self) -> None:"
    },
    {
      "line": 161,
      "text": "def work():"
    },
    {
      "line": 170,
      "text": "def _on_dlssrunning(self, now: dict) -> None:"
    },
    {
      "line": 181,
      "text": "def dlss_due(self) -> bool:"
    },
    {
      "line": 195,
      "text": "def _dlss_read(self, g, fresh: bool, procs=None) -> dict:"
    },
    {
      "line": 206,
      "text": "def dlss_reread(self, g) -> None:"
    },
    {
      "line": 224,
      "text": "def work():"
    },
    {
      "line": 241,
      "text": "def _on_dlssone(self, payload) -> None:"
    },
    {
      "line": 258,
      "text": "def dlss_scan(self, fresh: bool = False) -> None:"
    },
    {
      "line": 288,
      "text": "def work():"
    },
    {
      "line": 318,
      "text": "def _dlss_after_library(self, fresh: bool) -> None:"
    },
    {
      "line": 325,
      "text": "def again():"
    },
    {
      "line": 338,
      "text": "def _on_dlssseen(self, payload) -> None:"
    },
    {
      "line": 344,
      "text": "def _on_dlssscanned(self, data: dict) -> None:"
    },
    {
      "line": 378,
      "text": "def _dlss_can_start(self) -> bool:"
    },
    {
      "line": 386,
      "text": "def dlss_update(self, g, families=None) -> None:"
    },
    {
      "line": 406,
      "text": "def dlss_restore(self, g) -> None:"
    },
    {
      "line": 411,
      "text": "def dlss_update_from_game(self, g) -> None:"
    },
    {
      "line": 424,
      "text": "def dlss_update_all(self) -> None:"
    },
    {
      "line": 441,
      "text": "def _dlss_start(self, g, what: str, families, allow: bool) -> None:"
    },
    {
      "line": 452,
      "text": "def prog(done, total):"
    },
    {
      "line": 458,
      "text": "def work():"
    },
    {
      "line": 474,
      "text": "def _on_dlssprog(self, payload) -> None:"
    },
    {
      "line": 482,
      "text": "def _on_dlssdone(self, payload) -> None:"
    },
    {
      "line": 493,
      "text": "def _dlss_done(self, payload) -> None:"
    }
  ]
}

```text

"""Keeping games' DLSS current: the scan, the update and the restore behind
the dlss page. Logic only - dlsspage draws it.

Every folder walk, download and file swap runs on a worker; the page reads
the last scan from %LOCALAPPDATA%\\dlss5-autopilot\\dlss-scan.json, so it opens
with its rows at once and is usable while a new scan runs.
"""
from __future__ import annotations

import os
import threading
import time
from pathlib import Path

from .. import dlssupdate as du
from .. import log, watch

# A scan older than this is run again when the page opens: NVIDIA publishes
# a few builds a year, and a launcher can put an old file back any day.
RESCAN_AFTER = 24 * 3600


def _first_line(text: str) -> str:
    return (str(text).strip().splitlines() or [""])[0]


class DlssControl:
    # ------------------------------------------------------------ state
    def _dlss_init(self) -> None:
        self.dlss_data: dict | None = None     # the last scan, loaded on first use
        self.dlss_scanning = False
        self.dlss_seen = (0, 0, "")            # (games read, of, the one being read)
        self.dlss_job: str | None = None       # folder being updated or restored
        self.dlss_pct = -1                     # download progress of that job
        self.dlss_queue: list[str] = []        # folders "update all" still has to do
        self.dlss_results: dict[str, tuple[bool, str]] = {}
        self.dlss_pending: str | None = None    # a game page's update waiting for a scan to end
        self.dlss_batch: dict | None = None     # what "update all" did so far
        self.dlss_reread_q: list = []           # games dlss_reread still has to read
        self._dlss_rereading = False            # the one worker draining that queue
        self._dlss_rq_lock = threading.Lock()
        self._dlss_late: dict[str, dict] = {}   # rows re-read while a scan of every game ran
        self._dlss_waiting = False              # a read waits for the library scan to end
        self._dlss_wait_fresh = False

    def dlss_state(self) -> dict:
        if self.dlss_data is None:
            self.dlss_data = du.load_cache()
        return self.dlss_data

    def _dlss_games(self) -> list:
        # A game hidden in the library is one the person set aside: it is not
        # listed here and "update all" does not change its files.
        try:
            hid = self.hidden()
        except Exception:
            hid = set()
        return [g for g in self.all_games if getattr(g, "kind", "game") != "video" and g.exe
                and str(g.folder) not in hid]

    @staticmethod
    def _dlss_stamp(g) -> float:
        """The newest mtime of the game's folder and install folder. Worker only.

        Cheap enough to take for every game, unlike the walk: a game whose
        stamp did not move since its last read is not read again when a read
        of the new games runs."""
        t = 0.0
        for d in {str(g.folder), str(g.install_dir)}:
            try:
                t = max(t, os.stat(d).st_mtime)
            except OSError:
                pass
        return t

    @staticmethod
    def _dlss_entry(folder: str, d: dict) -> du.Entry:
        return du.Entry(path=Path(folder) / str(d.get("rel", "")), rel=str(d.get("rel", "")),
                        family=d.get("family") if d.get("family") in du.FILE_OF else "dlss",
                        version=str(d.get("version") or ""), state=str(d.get("state") or du.ORIGINAL),
                        original=str(d.get("original") or ""), label=str(d.get("label") or ""),
                        backup=Path("backup") if d.get("backup") else None)

    def dlss_newest(self) -> dict:
        n = self.dlss_state().get("newest")
        return n if isinstance(n, dict) else {}

    def dlss_row(self, g) -> dict | None:
        """What the last scan said about this game, with the page's verdicts added."""
        info = (self.dlss_state().get("games") or {}).get(str(g.folder))
        if not isinstance(info, dict) or not info.get("entries"):
            return None
        news = self.dlss_newest()
        entries = [self._dlss_entry(str(g.folder), d) for d in info["entries"] if isinstance(d, dict)]
        behind = [e for e in entries if du.outdated(e, news.get(e.family))]
        return {"g": g, "entries": entries, "behind": behind, "news": news,
                "anticheat": str(info.get("anticheat") or ""), "running": str(info.get("running") or ""),
                "anticheat_found": str(info.get("anticheat_found") or ""),
                "restorable": any(e.backup is not None or e.state == du.UPDATED for e in entries)}

    @staticmethod
    def _dlss_rank(r) -> tuple:
        # what can be updated now first, then what is behind but needs a look
        # (anti-cheat, running), then the rest
        now = bool(r["behind"] and not r["anticheat"] and not r["running"])
        return (not now, not r["behind"], r["g"].name.lower())

    def dlss_rows(self) -> list[dict]:
        rows = [r for r in (self.dlss_row(g) for g in self._dlss_games()) if r is not None]
        # Ordered once per scan, then kept: a row that jumps down the moment
        # its update ends (and again for every game of "update all") loses
        # the person's place. New rows go after the ordered ones.
        order = {f: i for i, f in enumerate(self.dlss_state().get("order") or [])}
        rows.sort(key=lambda r: (order.get(str(r["g"].folder), len(order)), self._dlss_rank(r)))
        return rows

    def dlss_todo(self) -> list[dict]:
        """What "update all" takes: behind, no anti-cheat, not running."""
        return [r for r in self.dlss_rows() if r["behind"] and not r["anticheat"] and not r["running"]]

    def dlss_line(self, g) -> str:
        """The game page's line, when this game's DLSS is behind: "dlss 3.7.20 -> 310.9.1"."""
        try:
            r = self.dlss_row(g)
        except Exception:
            return ""
        if not r or not r["behind"]:
            return ""
        e = r["behind"][0]
        new = r["news"].get(e.family, {}).get("version", "")
        n = len({x.family for x in r["behind"]}) - 1
        more = f" and {n} more" if n > 0 else ""
        name = {"dlss": "dlss", "dlssg": "frame generation", "dlssd": "ray reconstruction"}[e.family]
        held = f" - {r['anticheat'].lower()}" if r["anticheat"] else ""
        return f"{name} {e.version} -> {new}{more}{held}"

    def dlss_line_quiet(self, g) -> bool:
        """An anti-cheat game's line is drawn without the accent: not a suggestion."""
        r = self.dlss_row(g)
        return bool(r and r["anticheat"])

    def dlss_background(self) -> None:
        """At start: read the games quietly when the last read is old, so a
        game page can point out an older DLSS without the dlss page ever
        having been opened."""
        try:
            if self.scanning or self.busy:
                # the library is still being read, or an install runs: later
                self.root.after(15000, self.dlss_background)
            elif self.dlss_due():
                self.dlss_scan()
        except Exception:
            log.exception("starting the dlss read")

    def dlss_recheck_running(self) -> None:
        """Which listed games are running now - one process snapshot, on a worker."""
        rows = [r for r in self.dlss_rows()]
        if not rows or self.dlss_scanning:
            return

        def work():
            try:
                procs = watch.procs()
                now = {str(r["g"].folder): du.running(r["g"], procs) for r in rows}
            except Exception:
                return
            self.q.put(("dlssrunning", now))
        threading.Thread(target=work, daemon=True).start()

    def _on_dlssrunning(self, now: dict) -> None:
        games_ = self.dlss_state().get("games") or {}
        changed = False
        for folder, name in now.items():
            info = games_.get(folder)
            if isinstance(info, dict) and (info.get("running") or "") != name:
                info["running"] = name
                changed = True
        if changed:
            self.refresh("dlss")

    def dlss_due(self) -> bool:
        """Should opening the page start a scan by itself?"""
        if self.dlss_scanning or not self.have_library():
            return False
        data = self.dlss_state()
        if not data.get("at") or time.time() - float(data.get("at") or 0) > RESCAN_AFTER:
            return True
        seen = set(data.get("seen") or [])
        # A game whose bitness is not read yet (the library is enriching it)
        # cannot be read either; it becomes due once it is known.
        return any(str(g.folder) not in seen for g in self._dlss_games()
                   if getattr(g, "bitness", None) is not None)

    # ------------------------------------------------------------ scan
    def _dlss_read(self, g, fresh: bool, procs=None) -> dict:
        # No settle() here: a scan runs beside installs and other jobs, and
        # settle changes files. update() and restore() settle under busy.
        notes: list[str] = []
        stamp = self._dlss_stamp(g)
        entries = du.scan(g, fresh=fresh)
        ac, found = du.anticheat_info(g) if entries else ("", "")
        return {"name": g.name, "at": time.time(), "entries": [e.to_json() for e in entries],
                "anticheat": ac, "anticheat_found": found, "stamp": stamp,
                "running": du.running(g, procs) if entries else "", "notes": notes}

    def dlss_reread(self, g) -> None:
        """One game read again after something changed its files (a DLSS 5
        install or uninstall, autopilot, a library-wide update), so its page
        and row do not show the build from before.

        Called for many games in a row: they queue, and one worker reads them
        one after another instead of a thread and a walk per call at once. A
        game already queued is not queued twice."""
        if g is None or getattr(g, "bitness", None) != 64 or not self.dlss_state().get("games"):
            return
        with self._dlss_rq_lock:
            if any(str(x.folder) == str(g.folder) for x in self.dlss_reread_q):
                return
            self.dlss_reread_q.append(g)
            if self._dlss_rereading:
                return
            self._dlss_rereading = True

        def work():
            while True:
                # the queue and the flag change together, so a game queued
                # while the worker is finishing is never left unread
                with self._dlss_rq_lock:
                    if not self.dlss_reread_q:
                        self._dlss_rereading = False
                        return
                    one = self.dlss_reread_q.pop(0)
                try:
                    row = self._dlss_read(one, True)
                except Exception:
                    log.exception(f"reading the DLSS files of {one.name} again")
                    continue
                self.q.put(("dlssone", (str(one.folder), row)))
        threading.Thread(target=work, daemon=True).start()

    def _on_dlssone(self, payload) -> None:
        folder, row = payload
        data = self.dlss_state()
        if not isinstance(data.get("games"), dict):
            return
        if self.dlss_scanning:
            # the scan's answer replaces the whole table and may have read
            # this game before its files changed: this row goes on top of it
            self._dlss_late[folder] = row
        data["games"][folder] = row
        try:
            du.save_cache(data)
        except Exception:
            log.exception("saving the dlss read")
        self.refresh("dlss")
        self.refresh("game", soft=True)

    def dlss_scan(self, fresh: bool = False) -> None:
        if self.dlss_scanning or self.dlss_job:
            return
        if self.busy:
            self.shell.status("another job is running - the games are read when it ends")
            return
        if getattr(self, "scanning", False):
            # The library scan re-reads games while this would read them: a
            # game mid-enrich has no bitness yet, and was counted "32-bit
            # skipped" for a day. Read once the library is done.
            self.shell.status("the library is being read - the dlss files are read when it ends")
            self._dlss_after_library(fresh)
            return
        gs = self._dlss_games()
        if not gs:
            return
        data = self.dlss_state()
        # Only the day-old read and "check again" read every game. A read
        # started because the library has new games reads those, and the
        # games whose folder changed since their last read.
        partial = (not fresh and bool(data.get("at")) and isinstance(data.get("games"), dict)
                   and time.time() - float(data.get("at") or 0) <= RESCAN_AFTER)
        seen_before = set(data.get("seen") or []) if partial else set()
        stamps = {f: (v.get("stamp") if isinstance(v, dict) else None)
                  for f, v in (data.get("games") or {}).items()} if partial else {}
        self.dlss_scanning = True
        self._dlss_late = {}
        self.dlss_seen = (0, len(gs), "")
        self.refresh("dlss")

        def work():
            news = {}
            try:
                news = {f: {"label": e.get("label", ""), "version": e.get("version", "")}
                        for f, e in du.newest(refresh=fresh).items()}
            except Exception:
                log.exception("reading NVIDIA's newest DLSS builds")
            try:
                procs = watch.procs()
            except Exception:
                procs = []
            known = [g for g in gs if getattr(g, "bitness", None) is not None]
            x86 = len([g for g in known if g.bitness != 64])
            todo = [g for g in known if g.bitness == 64]
            if partial:
                todo = [g for g in todo if str(g.folder) not in seen_before
                        or stamps.get(str(g.folder)) != self._dlss_stamp(g)]
            out = {}
            for i, g in enumerate(todo, 1):
                self.q.put(("dlssseen", (i - 1, len(todo), g.name)))
                try:
                    out[str(g.folder)] = self._dlss_read(g, fresh, procs)
                except Exception:
                    log.exception(f"reading the DLSS files of {g.name}")
            # an unread bitness is neither checked nor skipped: not in `seen`,
            # so the game is read once the library knows it
            self.q.put(("dlssscanned", {"at": time.time(), "newest": news, "games": out, "x86": x86,
                                        "seen": [str(g.folder) for g in known], "partial": partial}))
        threading.Thread(target=work, daemon=True).start()

    def _dlss_after_library(self, fresh: bool) -> None:
        """One read, started when the library scan ends - however often asked."""
        self._dlss_wait_fresh = self._dlss_wait_fresh or fresh
        if self._dlss_waiting:
            return
        self._dlss_waiting = True

        def again():
            if getattr(self, "scanning", False):
                self.root.after(2000, again)
                return
            self._dlss_waiting = False
            f, self._dlss_wait_fresh = self._dlss_wait_fresh, False
            try:
                if f or self.dlss_due():
                    self.dlss_scan(fresh=f)
            except Exception:
                log.exception("reading the dlss files after the library scan")
        self.root.after(2000, again)

    def _on_dlssseen(self, payload) -> None:
        self.dlss_seen = payload
        page = self.shell.pages.get("dlss")
        if self.shell.page is page and hasattr(page, "progress"):
            page.progress()

    def _on_dlssscanned(self, data: dict) -> None:
        old = self.dlss_state()
        if not data.get("newest"):
            # offline: the rows are new, the newest builds are what was known
            data["newest"] = old.get("newest") or {}
            data["offline"] = True
        if data.pop("partial", False):
            # the games read now go over the last full read, which keeps its
            # time: the day-old read of every game still comes when it is due
            rows = dict(old.get("games") or {})
            rows.update(data["games"])
            data["games"] = rows
            data["at"] = old.get("at") or data["at"]
        # rows a re-read took while this ran are newer than what it read
        data["games"].update(self._dlss_late)
        self._dlss_late = {}
        self.dlss_data = data
        self.dlss_scanning = False
        data["order"] = []
        data["order"] = [str(r["g"].folder) for r in sorted(self.dlss_rows(), key=self._dlss_rank)]
        du.save_cache(data)
        n = len([1 for v in data["games"].values() if v.get("entries")])
        behind = len([r for r in self.dlss_rows() if r["behind"]])
        self.shell.status(f"dlss: {n} game{'s' if n != 1 else ''} ship it"
                          + (f", {behind} behind nvidia's newest" if behind else ""))
        self.refresh("dlss")
        self.refresh("game")
        # a game page's "update" pressed while this read was running
        pending, self.dlss_pending = self.dlss_pending, None
        g = next((x for x in self._dlss_games() if str(x.folder) == pending), None) if pending else None
        if g is not None:
            self.dlss_update(g)

    # ------------------------------------------------------------ update / restore
    def _dlss_can_start(self) -> bool:
        if self.dlss_job or self.dlss_scanning:
            return False
        if self.busy:
            self.shell.status("another job is running - the dlss files wait for it")
            return False
        return True

    def dlss_update(self, g, families=None) -> None:
        if not self._dlss_can_start():
            return
        r = self.dlss_row(g) or {}
        allow = False
        if r.get("anticheat"):
            if not self.shell.ask(
                    f"{r['anticheat']} - update anyway?",
                    f"{g.name} ships {r['anticheat']}"
                    + (f" ({r['anticheat_found']})" if r.get("anticheat_found") else "")
                    + ". An anti-cheat can read a changed DLSS file as "
                    f"tampering, and online that can mean a ban. The game's own files are kept either way.",
                    "update anyway", "cancel", danger=True):
                return
            allow = True
            # the question is modal, but timers are not: a job may have begun meanwhile
            if not self._dlss_can_start():
                return
        self._dlss_start(g, "update", families, allow)

    def dlss_restore(self, g) -> None:
        if not self._dlss_can_start():
            return
        self._dlss_start(g, "restore", None, False)

    def dlss_update_from_game(self, g) -> None:
        """The game page's link: the dlss page, with this game's update running on it.

        Opening the page can start a read of every game (a day-old read, a
        new game); the update then waits for it instead of being dropped.
        """
        self.shell.show("dlss")
        if self.dlss_scanning:
            self.dlss_pending = str(g.folder)
            self.shell.status(f"{g.name}: the update starts when the games are read")
            return
        self.dlss_update(g)

    def dlss_update_all(self) -> None:
        if not self._dlss_can_start():
            return
        todo = self.dlss_todo()
        if not todo:
            return
        names = ", ".join(r["g"].name for r in todo[:4]) + (f" and {len(todo) - 4} more" if len(todo) > 4 else "")
        if not self.shell.ask(f"update dlss in {len(todo)} game{'s' if len(todo) != 1 else ''}?",
                              f"{names}.\n\nEach game's own files are kept beside the new ones; "
                              f"restore original puts them back.", "update", "cancel"):
            return
        if not self._dlss_can_start():
            return
        self.dlss_queue = [str(r["g"].folder) for r in todo[1:]]
        self.dlss_batch = {"done": 0, "failed": []}
        self._dlss_start(todo[0]["g"], "update", None, False)

    def _dlss_start(self, g, what: str, families, allow: bool) -> None:
        folder = str(g.folder)
        self.dlss_job, self.dlss_pct = folder, -1
        # The same flag install, autopilot and uninstall wait for: two writers
        # of one runtime at once is how the game's own file gets lost.
        self.busy, self.action = True, "dlss"
        self.dlss_results.pop(folder, None)
        self.shell.busy(f"{'updating' if what == 'update' else 'restoring'} dlss in {g.name}...")
        self.refresh("dlss")
        last = [0.0]

        def prog(done, total):
            now = time.monotonic()
            if total and (now - last[0] > 0.15 or done >= total):
                last[0] = now
                self.q.put(("dlssprog", (folder, int(done * 100 / total))))

        def work():
            try:
                if what == "update":
                    rep = du.update(g, families, allow_anticheat=allow, progress=prog)
                else:
                    rep = du.restore(g)
            except Exception as e:
                log.exception(f"dlss {what} in {g.name}", e)
                rep = du.Report(error=_first_line(e) or type(e).__name__)
            try:
                row = self._dlss_read(g, fresh=False)
            except Exception:
                row = None
            self.q.put(("dlssdone", (folder, what, rep, row)))
        threading.Thread(target=work, daemon=True).start()

    def _on_dlssprog(self, payload) -> None:
        folder, pct = payload
        if folder == self.dlss_job:
            self.dlss_pct = pct
            page = self.shell.pages.get("dlss")
            if self.shell.page is page and hasattr(page, "job_progress"):
                page.job_progress()

    def _on_dlssdone(self, payload) -> None:
        try:
            self._dlss_done(payload)
        except Exception:
            # whatever went wrong drawing the answer, the job is over: a
            # "working" row and a stuck flag would block every later job
            self.dlss_job, self.dlss_pct, self.dlss_queue, self.dlss_batch = None, -1, [], None
            self.busy, self.action = False, ""
            self.shell.busy("")
            raise

    def _dlss_done(self, payload) -> None:
        folder, what, rep, row = payload
        data = self.dlss_state()
        if row is not None:
            if rep.refused == du.R_RUNNING:
                row["running"] = row.get("running") or rep.error.split(" is running")[0]
            data.setdefault("games", {})[folder] = row
            du.save_cache(data)
        name = (row or {}).get("name") or Path(folder).name
        if rep.ok:
            # The row's own lines already show the versions; what the row adds
            # is a note the person has to know (a launcher put its file back,
            # a backup was gone), else one word.
            done = ("updated" if what == "update" else "the game's own files are back") if rep.done else ""
            if done:
                text = done
            elif rep.notes:
                text = rep.notes[0]
            else:
                text = (rep.skipped or ["nothing to change"])[0]
            self.write(f"{name}: " + "; ".join(rep.done + rep.notes + rep.skipped), "ok")
        else:
            # the row names the file inside the game; the log keeps the whole path
            text = rep.error.replace(folder + os.sep, "").replace(folder + "/", "").replace(folder, "the game folder")
            if rep.done:
                # part of it went through: say so, above the part that did not
                moved = ", ".join(line.split("  ")[0] for line in rep.done)
                text = f"{moved} updated; {text}"
            self.write(f"{name}: " + "; ".join(rep.done + [rep.error]), "err")
        self.dlss_results[folder] = (rep.ok, text)
        self.busy, self.action = False, ""
        if self.dlss_batch is not None:
            if rep.ok and rep.done:
                self.dlss_batch["done"] += 1
            elif not rep.ok:
                self.dlss_batch["failed"].append(name)
        self.dlss_job, self.dlss_pct = None, -1
        nxt = None
        while self.dlss_queue and nxt is None:
            f = self.dlss_queue.pop(0)
            nxt = next((g for g in self._dlss_games() if str(g.folder) == f), None)
        if nxt is not None:
            self._dlss_start(nxt, "update", None, False)
            return
        self.shell.busy("")
        batch, self.dlss_batch = self.dlss_batch, None
        if batch is not None:
            failed = batch["failed"]
            self.shell.status(f"dlss: {batch['done']} updated"
                              + (f", {len(failed)} failed - {', '.join(failed[:3])}" if failed else ""))
        else:
            self.shell.status(f"{name}: {_first_line(text)}")
        self.refresh("dlss")
        self.refresh("game")


```
