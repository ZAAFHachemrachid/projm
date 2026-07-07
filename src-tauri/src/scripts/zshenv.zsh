# projm-shell-integration (zshenv)
#
# Trailing `:` is load-bearing — without it, a missing user .zshenv leaves $?=1,
# which propagates through the rest of init and ultimately into the first
# prompt's `%?` (rendering robbyrussell's `➜` red on a clean shell start).
#
# ZDOTDIR is swapped to the user's value while their file runs — configs
# commonly reference "$ZDOTDIR/…" (XDG setups keep everything under
# ~/.config/zsh) and must not see the shim directory — then swapped back so
# zsh finds the next shim.
{
  _projm_shim_zdotdir="$ZDOTDIR"
  if [ -n "${PROJM_USER_ZDOTDIR:-}" ]; then
    ZDOTDIR="$PROJM_USER_ZDOTDIR"
  else
    unset ZDOTDIR
  fi
  _projm_user_zdotdir="${PROJM_USER_ZDOTDIR:-$HOME}"
  [ -f "$_projm_user_zdotdir/.zshenv" ] && source "$_projm_user_zdotdir/.zshenv"
  # A GUI-launched app has no ZDOTDIR to capture at spawn, but the user's
  # ~/.zshenv may relocate it (ZDOTDIR=~/.config/zsh). Capture that so the
  # remaining shims source config from the right place.
  if [ -n "${ZDOTDIR:-}" ] && [ "$ZDOTDIR" != "$_projm_shim_zdotdir" ]; then
    export PROJM_USER_ZDOTDIR="$ZDOTDIR"
  fi
  ZDOTDIR="$_projm_shim_zdotdir"
  unset _projm_shim_zdotdir _projm_user_zdotdir
}
:
