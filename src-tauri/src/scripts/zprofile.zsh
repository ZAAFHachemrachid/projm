# projm-shell-integration (zprofile)
#
# See zshenv.zsh for the rationale on the trailing `:` and the ZDOTDIR swap.
{
  _projm_shim_zdotdir="$ZDOTDIR"
  if [ -n "${PROJM_USER_ZDOTDIR:-}" ]; then
    ZDOTDIR="$PROJM_USER_ZDOTDIR"
  else
    unset ZDOTDIR
  fi
  _projm_user_zdotdir="${PROJM_USER_ZDOTDIR:-$HOME}"
  [ -f "$_projm_user_zdotdir/.zprofile" ] && source "$_projm_user_zdotdir/.zprofile"
  ZDOTDIR="$_projm_shim_zdotdir"
  unset _projm_shim_zdotdir _projm_user_zdotdir
}
:
