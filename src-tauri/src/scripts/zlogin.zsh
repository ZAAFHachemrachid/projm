# projm-shell-integration (zlogin)
#
# This is the LAST init file zsh runs before entering the prompt loop, so its
# exit status becomes `$?` for the very first prompt. Without the trailing `:`,
# users without a personal ~/.zlogin (the common case) hit a non-zero $? on
# first render — themes that condition prompt color on `%?` (robbyrussell etc.)
# show a red error indicator on a clean shell start.
# Last init file: restore the user's ZDOTDIR permanently. Leaving the shim
# directory exported would make every nested zsh (editors, tools, `zsh` typed
# at the prompt) source shim config instead of the user's.
{
  _projm_user_zdotdir="${PROJM_USER_ZDOTDIR:-$HOME}"
  if [ -n "${PROJM_USER_ZDOTDIR:-}" ]; then
    ZDOTDIR="$PROJM_USER_ZDOTDIR"
  else
    unset ZDOTDIR
  fi
  [ -f "$_projm_user_zdotdir/.zlogin" ] && source "$_projm_user_zdotdir/.zlogin"
  unset _projm_user_zdotdir
}
:
