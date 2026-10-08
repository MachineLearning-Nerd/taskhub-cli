_taskhub() {
    local i cur prev opts cmd
    COMPREPLY=()
    if [[ "${BASH_VERSINFO[0]}" -ge 4 ]]; then
        cur="$2"
    else
        cur="${COMP_WORDS[COMP_CWORD]}"
    fi
    prev="$3"
    cmd=""
    opts=""

    for i in "${COMP_WORDS[@]:0:COMP_CWORD}"
    do
        case "${cmd},${i}" in
            ",$1")
                cmd="taskhub"
                ;;
            taskhub,attachments)
                cmd="taskhub__subcmd__attachments"
                ;;
            taskhub,auth)
                cmd="taskhub__subcmd__auth"
                ;;
            taskhub,branch)
                cmd="taskhub__subcmd__branch"
                ;;
            taskhub,claim)
                cmd="taskhub__subcmd__claim"
                ;;
            taskhub,comment)
                cmd="taskhub__subcmd__comment"
                ;;
            taskhub,comments)
                cmd="taskhub__subcmd__comments"
                ;;
            taskhub,completions)
                cmd="taskhub__subcmd__completions"
                ;;
            taskhub,guide)
                cmd="taskhub__subcmd__guide"
                ;;
            taskhub,inbox)
                cmd="taskhub__subcmd__inbox"
                ;;
            taskhub,items)
                cmd="taskhub__subcmd__items"
                ;;
            taskhub,link)
                cmd="taskhub__subcmd__link"
                ;;
            taskhub,man)
                cmd="taskhub__subcmd__man"
                ;;
            taskhub,mcp)
                cmd="taskhub__subcmd__mcp"
                ;;
            taskhub,mine)
                cmd="taskhub__subcmd__mine"
                ;;
            taskhub,next)
                cmd="taskhub__subcmd__next"
                ;;
            taskhub,open)
                cmd="taskhub__subcmd__open"
                ;;
            taskhub,pending)
                cmd="taskhub__subcmd__pending"
                ;;
            taskhub,projects)
                cmd="taskhub__subcmd__projects"
                ;;
            taskhub,reject)
                cmd="taskhub__subcmd__reject"
                ;;
            taskhub,retry)
                cmd="taskhub__subcmd__retry"
                ;;
            taskhub,show)
                cmd="taskhub__subcmd__show"
                ;;
            taskhub,skill)
                cmd="taskhub__subcmd__skill"
                ;;
            taskhub,submit)
                cmd="taskhub__subcmd__submit"
                ;;
            taskhub,version)
                cmd="taskhub__subcmd__version"
                ;;
            taskhub__subcmd__attachments,add)
                cmd="taskhub__subcmd__attachments__subcmd__add"
                ;;
            taskhub__subcmd__attachments,download)
                cmd="taskhub__subcmd__attachments__subcmd__download"
                ;;
            taskhub__subcmd__attachments,list)
                cmd="taskhub__subcmd__attachments__subcmd__list"
                ;;
            taskhub__subcmd__auth,login)
                cmd="taskhub__subcmd__auth__subcmd__login"
                ;;
            taskhub__subcmd__auth,logout)
                cmd="taskhub__subcmd__auth__subcmd__logout"
                ;;
            taskhub__subcmd__auth,status)
                cmd="taskhub__subcmd__auth__subcmd__status"
                ;;
            taskhub__subcmd__comments,edit)
                cmd="taskhub__subcmd__comments__subcmd__edit"
                ;;
            taskhub__subcmd__comments,list)
                cmd="taskhub__subcmd__comments__subcmd__list"
                ;;
            taskhub__subcmd__inbox,done)
                cmd="taskhub__subcmd__inbox__subcmd__done"
                ;;
            taskhub__subcmd__items,activity)
                cmd="taskhub__subcmd__items__subcmd__activity"
                ;;
            taskhub__subcmd__items,create)
                cmd="taskhub__subcmd__items__subcmd__create"
                ;;
            taskhub__subcmd__items,get)
                cmd="taskhub__subcmd__items__subcmd__get"
                ;;
            taskhub__subcmd__items,list)
                cmd="taskhub__subcmd__items__subcmd__list"
                ;;
            taskhub__subcmd__items,move)
                cmd="taskhub__subcmd__items__subcmd__move"
                ;;
            taskhub__subcmd__items,update)
                cmd="taskhub__subcmd__items__subcmd__update"
                ;;
            taskhub__subcmd__pending,discard)
                cmd="taskhub__subcmd__pending__subcmd__discard"
                ;;
            taskhub__subcmd__projects,list)
                cmd="taskhub__subcmd__projects__subcmd__list"
                ;;
            taskhub__subcmd__projects,show)
                cmd="taskhub__subcmd__projects__subcmd__show"
                ;;
            taskhub__subcmd__skill,install)
                cmd="taskhub__subcmd__skill__subcmd__install"
                ;;
            *)
                ;;
        esac
    done

    case "${cmd}" in
        taskhub)
            opts="-h -V --json --human --help --version next claim show mine comment submit reject inbox link branch open items comments attachments projects pending retry auth guide skill mcp completions version man"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 1 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        taskhub__subcmd__attachments)
            opts="-h --json --human --help list add download"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 2 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        taskhub__subcmd__attachments__subcmd__add)
            opts="-h --request-id --json --human --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --request-id)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        taskhub__subcmd__attachments__subcmd__download)
            opts="-o -h --output --force --json --human --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --output)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                -o)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        taskhub__subcmd__attachments__subcmd__list)
            opts="-h --limit --cursor --json --human --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --limit)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --cursor)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        taskhub__subcmd__auth)
            opts="-h --json --human --help login status logout"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 2 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        taskhub__subcmd__auth__subcmd__login)
            opts="-h --with-token --origin --json --human --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --origin)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        taskhub__subcmd__auth__subcmd__logout)
            opts="-h --json --human --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        taskhub__subcmd__auth__subcmd__status)
            opts="-h --json --human --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        taskhub__subcmd__branch)
            opts="-h --create --json --human --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 2 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        taskhub__subcmd__claim)
            opts="-h --request-id --json --human --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 2 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --request-id)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        taskhub__subcmd__comment)
            opts="-h --body --body-file --attach --request-id --json --human --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 2 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --body)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --body-file)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --attach)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --request-id)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        taskhub__subcmd__comments)
            opts="-h --json --human --help list edit"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 2 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        taskhub__subcmd__comments__subcmd__edit)
            opts="-h --body --body-file --request-id --json --human --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --body)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --body-file)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --request-id)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        taskhub__subcmd__comments__subcmd__list)
            opts="-h --limit --cursor --json --human --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --limit)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --cursor)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        taskhub__subcmd__completions)
            opts="-h --json --human --help bash elvish fish powershell zsh"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 2 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        taskhub__subcmd__guide)
            opts="-h --json --human --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 2 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        taskhub__subcmd__inbox)
            opts="-h --unread --kind --limit --cursor --json --human --help done"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 2 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --kind)
                    COMPREPLY=($(compgen -W "mention assigned rejected review" -- "${cur}"))
                    return 0
                    ;;
                --limit)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --cursor)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        taskhub__subcmd__inbox__subcmd__done)
            opts="-h --all --request-id --json --human --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --request-id)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        taskhub__subcmd__items)
            opts="-h --json --human --help list get activity create update move"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 2 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        taskhub__subcmd__items__subcmd__activity)
            opts="-h --limit --cursor --json --human --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --limit)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --cursor)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        taskhub__subcmd__items__subcmd__create)
            opts="-h --project --type --title --description --body-file --priority --assignee --label --input-file --request-id --json --human --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --project)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --type)
                    COMPREPLY=($(compgen -W "task bug" -- "${cur}"))
                    return 0
                    ;;
                --title)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --description)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --body-file)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --priority)
                    COMPREPLY=($(compgen -W "high medium low" -- "${cur}"))
                    return 0
                    ;;
                --assignee)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --label)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --input-file)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --request-id)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        taskhub__subcmd__items__subcmd__get)
            opts="-h --json --human --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        taskhub__subcmd__items__subcmd__list)
            opts="-h --project --type --status --priority --assignee --label --search --agent --sort --limit --cursor --json --human --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --project)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --type)
                    COMPREPLY=($(compgen -W "task bug" -- "${cur}"))
                    return 0
                    ;;
                --status)
                    COMPREPLY=($(compgen -W "todo in_progress dev_done done" -- "${cur}"))
                    return 0
                    ;;
                --priority)
                    COMPREPLY=($(compgen -W "high medium low" -- "${cur}"))
                    return 0
                    ;;
                --assignee)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --label)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --search)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --sort)
                    COMPREPLY=($(compgen -W "number priority" -- "${cur}"))
                    return 0
                    ;;
                --limit)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --cursor)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        taskhub__subcmd__items__subcmd__move)
            opts="-h --from --request-id --json --human --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --from)
                    COMPREPLY=($(compgen -W "todo in_progress dev_done done" -- "${cur}"))
                    return 0
                    ;;
                --request-id)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        taskhub__subcmd__items__subcmd__update)
            opts="-h --if-version --type --title --description --body-file --priority --assignee --label --no-labels --request-id --json --human --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --if-version)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --type)
                    COMPREPLY=($(compgen -W "task bug" -- "${cur}"))
                    return 0
                    ;;
                --title)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --description)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --body-file)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --priority)
                    COMPREPLY=($(compgen -W "high medium low" -- "${cur}"))
                    return 0
                    ;;
                --assignee)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --label)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --request-id)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        taskhub__subcmd__link)
            opts="-h --kind --request-id --json --human --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 2 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --kind)
                    COMPREPLY=($(compgen -W "pr link" -- "${cur}"))
                    return 0
                    ;;
                --request-id)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        taskhub__subcmd__man)
            opts="-h --json --human --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 2 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        taskhub__subcmd__mcp)
            opts="-h --json --human --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 2 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        taskhub__subcmd__mine)
            opts="-h --status --json --human --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 2 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --status)
                    COMPREPLY=($(compgen -W "todo in_progress dev_done done" -- "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        taskhub__subcmd__next)
            opts="-h --project --type --claim --json --human --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 2 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --project)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --type)
                    COMPREPLY=($(compgen -W "task bug" -- "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        taskhub__subcmd__open)
            opts="-h --request-id --json --human --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 2 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --request-id)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        taskhub__subcmd__pending)
            opts="-h --json --human --help discard"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 2 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        taskhub__subcmd__pending__subcmd__discard)
            opts="-h --json --human --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        taskhub__subcmd__projects)
            opts="-h --json --human --help list show"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 2 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        taskhub__subcmd__projects__subcmd__list)
            opts="-h --json --human --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        taskhub__subcmd__projects__subcmd__show)
            opts="-h --json --human --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        taskhub__subcmd__reject)
            opts="-h --reason --reason-file --attach --request-id --json --human --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 2 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --reason)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --reason-file)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --attach)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --request-id)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        taskhub__subcmd__retry)
            opts="-h --json --human --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 2 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        taskhub__subcmd__show)
            opts="-h --comments --attachments --json --human --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 2 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --comments)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --attachments)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        taskhub__subcmd__skill)
            opts="-h --json --human --help install"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 2 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        taskhub__subcmd__skill__subcmd__install)
            opts="-h --client --project --json --human --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --client)
                    COMPREPLY=($(compgen -W "claude codex all" -- "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        taskhub__subcmd__submit)
            opts="-h --from --summary --summary-file --testing --testing-file --limitations --limitations-file --attach --pr --link --input-file --request-id --json --human --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 2 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --from)
                    COMPREPLY=($(compgen -W "todo in_progress dev_done done" -- "${cur}"))
                    return 0
                    ;;
                --summary)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --summary-file)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --testing)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --testing-file)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --limitations)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --limitations-file)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --attach)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --pr)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --link)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --input-file)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --request-id)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        taskhub__subcmd__version)
            opts="-h --json --human --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 2 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
    esac
}

if [[ "${BASH_VERSINFO[0]}" -eq 4 && "${BASH_VERSINFO[1]}" -ge 4 || "${BASH_VERSINFO[0]}" -gt 4 ]]; then
    complete -F _taskhub -o nosort -o bashdefault -o default taskhub
else
    complete -F _taskhub -o bashdefault -o default taskhub
fi
