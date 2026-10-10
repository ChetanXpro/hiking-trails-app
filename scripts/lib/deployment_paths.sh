#!/usr/bin/env bash

# Shared path validation for deployment artifact scripts. Callers must enable
# `set -e`; these helpers return non-zero on any containment violation.

validate_deployment_relative_path() {
	local value=$1 label=$2
	[[ -n "${value}" && "${value}" != /* && "${value}" != *$'\t'* && "${value}" != *$'\n'* && "${value}" != *$'\r'* ]] || {
		echo "Invalid ${label}: ${value}" >&2
		return 1
	}
	case "/${value}/" in
		*/../*|*/./*)
			echo "${label} may not contain . or .. components: ${value}" >&2
			return 1
			;;
	esac
}

deployment_root() {
	local root=$1 mode=${2:-existing}
	if [[ "${mode}" == create ]]; then
		if [[ -L "${root}" ]]; then
			echo "Deployment root may not be a symlink: ${root}" >&2
			return 1
		fi
		mkdir -p "${root}"
	fi
	[[ -d "${root}" && ! -L "${root}" ]] || {
		echo "Deployment root must be a real directory: ${root}" >&2
		return 1
	}
	(cd "${root}" && pwd -P)
}

assert_no_deployment_symlink_components() {
	local root_abs=$1 relative=$2 allow_create=${3:-false}
	local parent_relative current component candidate

	parent_relative=${relative%/*}
	[[ "${parent_relative}" != "${relative}" ]] || parent_relative=""
	current=${root_abs}

	while [[ -n "${parent_relative}" ]]; do
		component=${parent_relative%%/*}
		if [[ "${parent_relative}" == */* ]]; then
			parent_relative=${parent_relative#*/}
		else
			parent_relative=""
		fi
		[[ -n "${component}" ]] || continue

		candidate="${current}/${component}"
		if [[ -L "${candidate}" ]]; then
			echo "Deployment path contains a symlink: ${candidate}" >&2
			return 1
		fi
		if [[ ! -e "${candidate}" ]]; then
			if [[ "${allow_create}" == true ]]; then
				mkdir "${candidate}"
			else
				echo "Deployment path component does not exist: ${candidate}" >&2
				return 1
			fi
		fi
		[[ -d "${candidate}" ]] || {
			echo "Deployment path component is not a directory: ${candidate}" >&2
			return 1
		}
		current=$(cd "${candidate}" && pwd -P)
		case "${current}/" in
			"${root_abs}/"*) ;;
			*)
				echo "Deployment path escaped its root: ${relative}" >&2
				return 1
				;;
		esac
	done
}

deployment_existing_file() {
	local root=$1 relative=$2 label=${3:-asset}
	local root_abs candidate parent_abs

	validate_deployment_relative_path "${relative}" "${label}" || return 1
	root_abs=$(deployment_root "${root}") || return 1
	assert_no_deployment_symlink_components "${root_abs}" "${relative}" false || return 1
	candidate="${root_abs}/${relative}"
	[[ -f "${candidate}" && ! -L "${candidate}" ]] || {
		echo "${label} must be a regular, non-symlink file: ${candidate}" >&2
		return 1
	}
	parent_abs=$(cd "$(dirname "${candidate}")" && pwd -P) || return 1
	case "${parent_abs}/" in
		"${root_abs}/"*) ;;
		*)
			echo "${label} escaped its root: ${relative}" >&2
			return 1
			;;
	esac
	printf '%s\n' "${candidate}"
}

deployment_destination_file() {
	local root=$1 relative=$2 label=${3:-destination}
	local root_abs candidate parent_abs

	validate_deployment_relative_path "${relative}" "${label}" || return 1
	root_abs=$(deployment_root "${root}" create) || return 1
	assert_no_deployment_symlink_components "${root_abs}" "${relative}" true || return 1
	candidate="${root_abs}/${relative}"
	[[ ! -L "${candidate}" ]] || {
		echo "${label} may not be a symlink: ${candidate}" >&2
		return 1
	}
	[[ ! -e "${candidate}" || -f "${candidate}" ]] || {
		echo "${label} must be absent or a regular file: ${candidate}" >&2
		return 1
	}
	parent_abs=$(cd "$(dirname "${candidate}")" && pwd -P) || return 1
	case "${parent_abs}/" in
		"${root_abs}/"*) ;;
		*)
			echo "${label} escaped its root: ${relative}" >&2
			return 1
			;;
	esac
	printf '%s\n' "${candidate}"
}

assert_deployment_regular_or_absent() {
	local path=$1 label=$2
	[[ ! -L "${path}" ]] || {
		echo "${label} may not be a symlink: ${path}" >&2
		return 1
	}
	[[ ! -e "${path}" || -f "${path}" ]] || {
		echo "${label} must be absent or a regular file: ${path}" >&2
		return 1
	}
}
