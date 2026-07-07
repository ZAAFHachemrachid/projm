# projm-shell-integration (zprofile)
#
# See zshenv.zsh for the rationale on the trailing `:`.
{
  _projm_user_zdotdir="${PROJM_USER_ZDOTDIR:-$HOME}"
  [ -f "$_projm_user_zdotdir/.zprofile" ] && source "$_projm_user_zdotdir/.zprofile"
  unset _projm_user_zdotdir
}
:
