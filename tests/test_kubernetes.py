# code taken from https://gitlab.com/fluidattacks/universe/-/blob/trunk/skims/skims/lib_sast/root/f267/kubernetes.py
import atatus
from collections.abc import (
    Iterator,
)
from lib_sast.path.common import (
    TRUE_OPTIONS,
)
from lib_sast.root.common import (
    get_vulnerabilities_from_n_ids,
)
from lib_sast.root.utilities.kubernetes import (
    check_template_integrity_for_all_nodes,
    get_attr_inside_attrs,
    get_attribute,
    get_containers_capabilities,
    get_optional_attribute,
    get_pod_spec,
)
from model.core import (
    MethodExecutionResult,
    MethodsEnum,
)
from model.graph import (
    Graph,
    GraphShard,
    MethodSupplies,
    NId,
    QuerySupplies,
)
from utils.graph import (
    adj_ast,
    pred,
)


def _aux_iterate_containers(graph: Graph, nid: NId) -> Iterator[NId]:
    container_attrs = graph.nodes[nid]["value_id"]
    for c_id in adj_ast(graph, container_attrs):
        yield c_id


def get_host_pid(graph: Graph, nid: NId) -> Iterator[NId]:
    spec, _, spec_id = get_attribute(graph, nid, "spec")
    if spec:
        spec_attrs = graph.nodes[spec_id]["value_id"]
        hostpid_attrs = get_attribute(graph, spec_attrs, "hostPID")
        if hostpid_attrs[1] == "true":
            yield hostpid_attrs[2]


def iterate_containers(graph: Graph, nid: NId) -> Iterator[NId]:
    containers_type = {
        "containers",
        "ephemeralContainers",
        "initContainers",
    }

    spec, _, spec_id = get_attribute(graph, nid, "spec")
    if spec:
        spec_attrs = graph.nodes[spec_id]["value_id"]
        for cont in containers_type:
            container = get_attribute(graph, spec_attrs, cont)
            if container[0]:
                yield from _aux_iterate_containers(graph, container[2])


def _iterate_sec_ctx_from_spec(graph: Graph, nid: NId) -> Iterator[NId]:
    spec, _, spec_id = get_attribute(graph, nid, "spec")
    if spec:
        spec_attrs = graph.nodes[spec_id]["value_id"]
        sec_ctx = get_attribute(graph, spec_attrs, "securityContext")
        if sec_ctx[0]:
            sec_ctx_attrs = graph.nodes[sec_ctx[2]]["value_id"]
            yield sec_ctx_attrs
        else:
            yield spec_attrs


def iterate_security_context(
    graph: Graph, nid: NId, container_only: bool
) -> Iterator[NId]:
    if (
        not container_only
        and (kind := get_attribute(graph, nid, "kind"))
        and kind[1] == "Pod"
    ):
        yield from _iterate_sec_ctx_from_spec(graph, nid)
    for container in iterate_containers(graph, nid):
        sec_ctx = get_attribute(graph, container, "securityContext")
        if sec_ctx[0]:
            sec_ctx_attrs = graph.nodes[sec_ctx[2]]["value_id"]
            yield sec_ctx_attrs


def _container_without_securitycontext(graph: Graph, nid: NId) -> Iterator[NId]:
    pod_has_safe_config = False
    if spec := get_optional_attribute(graph, nid, "spec"):
        spec_attrs = graph.nodes[spec[2]]["value_id"]
        if get_optional_attribute(graph, spec_attrs, "securityContext"):
            pod_has_safe_config = True
        for container in iterate_containers(graph, nid):
            security_context = get_attribute(graph, container, "securityContext")
            if not security_context[0] and not pod_has_safe_config:
                yield container


def _check_drop_capability(graph: Graph, nid: NId) -> Iterator[NId]:
    cap_drop = get_containers_capabilities(graph, nid, "drop")
    if cap_drop and "all" not in [cap.lower() for cap in cap_drop]:
        report = get_attr_inside_attrs(graph, nid, ["capabilities", "drop"])
        yield report[2]


def _check_if_capability_exists(graph: Graph, nid: NId) -> Iterator[NId]:
    cap_drop = get_containers_capabilities(graph, nid, "drop")
    cap_add = get_containers_capabilities(graph, nid, "add")
    if not cap_drop and not cap_add:
        yield graph.nodes[pred(graph, nid)[0]]["key_id"]


def _check_if_sys_admin_exists(graph: Graph, nid: NId) -> Iterator[NId]:
    cap_add = get_containers_capabilities(graph, nid, "add")
    if cap_add and "sys_admin" in [cap.lower() for cap in cap_add]:
        report = get_attr_inside_attrs(graph, nid, ["capabilities", "add"])
        yield report[2]


def _check_privileged_used(graph: Graph, nid: NId) -> Iterator[NId]:
    privileged, p_val, p_id = get_attribute(graph, nid, "privileged")
    if privileged and p_val in TRUE_OPTIONS:
        yield p_id


def _check_run_as_user(graph: Graph, nid: NId) -> Iterator[NId]:
    run_as_user, rau_val, rau_id = get_attribute(graph, nid, "runAsUser")
    if run_as_user and int(rau_val) < 10000:
        yield rau_id


def _check_add_capability(graph: Graph, nid: NId) -> Iterator[NId]:
    cap_add = get_containers_capabilities(graph, nid, "add")
    if cap_add and cap_add[0].lower() not in {
        "net_bind_service",
        "null",
        "nil",
        "undefined",
        "sys_admin",  # This is checked in another method
    }:
        report = get_attr_inside_attrs(graph, nid, ["capabilities", "add"])
        yield report[2]


def get_seccomp_vuln_line(
    graph: Graph, container_ctx: NId, pod_has_safe_config: bool
) -> NId | None:
    sec_ctx = graph.nodes[container_ctx]["value_id"]
    container_seccomp_profile = get_attribute(graph, sec_ctx, "seccompProfile")
    if container_seccomp_profile[0]:
        cont_seccomp_attrs = graph.nodes[container_seccomp_profile[2]]["value_id"]
        container_type = get_attribute(graph, cont_seccomp_attrs, "type")
        if container_type[0]:
            if container_type[1].lower() == "unconfined":
                return container_type[2]
        elif not pod_has_safe_config:
            return container_seccomp_profile[2]
    elif not pod_has_safe_config:
        return container_ctx
    return None


def _k8s_check_container_seccomp(
    graph: Graph, container_props: NId, pod_has_safe_config: bool
) -> NId | None:
    sec_ctx = get_attribute(graph, container_props, "securityContext")
    if sec_ctx[0]:
        if vuln := get_seccomp_vuln_line(graph, sec_ctx[2], pod_has_safe_config):
            return vuln
    return None


def _check_seccomp_profile(graph: Graph, nid: NId) -> Iterator[NId]:
    pod_has_safe_config: bool = False
    if (
        (spec := get_pod_spec(graph, nid))
        and (
            p_type := get_attr_inside_attrs(
                graph, spec, ["securityContext", "seccompProfile", "type"]
            )
        )
        and p_type[0]
    ):
        if p_type[1].lower() == "unconfined":
            yield p_type[2]
        elif p_type[1].lower() in ["runtimedefault", "localhost"]:
            pod_has_safe_config = True
    for container in iterate_containers(graph, nid):
        if vuln := _k8s_check_container_seccomp(graph, container, pod_has_safe_config):
            yield vuln


def _root_filesystem_read_only(graph: Graph, nid: NId) -> NId | None:
    security_context = get_attribute(graph, nid, "securityContext")
    if security_context[0]:
        sec_ctx_attrs = graph.nodes[security_context[2]]["value_id"]
        read_perm = get_attribute(graph, sec_ctx_attrs, "readOnlyRootFilesystem")
        if read_perm[0]:
            if read_perm[1].lower() != "true":
                return read_perm[2]
        else:
            return security_context[2]
    return None


def _allow_privilege_escalation_enabled(graph: Graph, nid: NId) -> NId | None:
    security_context = get_attribute(graph, nid, "securityContext")
    if security_context[0]:
        sec_ctx_attrs = graph.nodes[security_context[2]]["value_id"]
        read_perm = get_attribute(graph, sec_ctx_attrs, "allowPrivilegeEscalation")
        if read_perm[0]:
            if read_perm[1].lower() != "false":
                return read_perm[2]
        else:
            return security_context[2]
    return None


def get_container_root_vuln_line(
    graph: Graph, container_ctx: NId, pod_has_safe_config: bool
) -> NId | None:
    sec_ctx = graph.nodes[container_ctx]["value_id"]
    root = get_attribute(graph, sec_ctx, "runAsNonRoot")
    if root[0]:
        if root[1].lower() != "true":
            return root[2]
    elif not pod_has_safe_config:
        return container_ctx
    return None


def _k8s_check_container_root(
    graph: Graph, container_props: NId, pod_has_safe_config: bool
) -> NId | None:
    sec_ctx = get_attribute(graph, container_props, "securityContext")
    if sec_ctx[0]:
        if vuln := get_container_root_vuln_line(graph, sec_ctx[2], pod_has_safe_config):
            return vuln
    return None


def _root_container(graph: Graph, nid: NId) -> Iterator[NId]:
    pod_has_safe_config: bool = False
    if (
        (spec := get_pod_spec(graph, nid))
        and (
            perm := get_attr_inside_attrs(
                graph, spec, ["securityContext", "runAsNonRoot"]
            )
        )
        and perm[0]
    ):
        if perm[1].lower() != "true":
            yield perm[2]
        elif perm[1].lower() == "true":
            pod_has_safe_config = True

    for container in iterate_containers(graph, nid):
        if vuln := _k8s_check_container_root(graph, container, pod_has_safe_config):
            yield vuln


@atatus.capture_span()
def k8s_root_container(
    shard: GraphShard, method_supplies: MethodSupplies
) -> MethodExecutionResult:
    method = MethodsEnum.K8S_ROOT_CONTAINER

    def n_ids() -> Iterator[NId]:
        graph = shard.syntax_graph
        integrity = check_template_integrity_for_all_nodes(
            graph, method_supplies.selected_nodes
        )
        if integrity:
            for nid in method_supplies.selected_nodes:
                for c_id in _root_container(graph, nid):
                    yield c_id

    return get_vulnerabilities_from_n_ids(
        n_ids=n_ids(),
        query_supplies=QuerySupplies(
            desc_key="f267.k8s_root_container",
            desc_params={},
            method=method,
            method_calls=len(method_supplies.selected_nodes),
            graph_shard=shard,
        ),
    )


@atatus.capture_span()
def k8s_allow_privilege_escalation_enabled(
    shard: GraphShard, method_supplies: MethodSupplies
) -> MethodExecutionResult:
    method = MethodsEnum.K8S_PRIVILEGE_ESCALATION_ENABLED

    def n_ids() -> Iterator[NId]:
        graph = shard.syntax_graph
        integrity = check_template_integrity_for_all_nodes(
            graph, method_supplies.selected_nodes
        )
        if integrity:
            for nid in method_supplies.selected_nodes:
                for c_id in iterate_containers(graph, nid):
                    if report := _allow_privilege_escalation_enabled(graph, c_id):
                        yield report

    return get_vulnerabilities_from_n_ids(
        n_ids=n_ids(),
        query_supplies=QuerySupplies(
            desc_key="f267.k8s_allow_privilege_escalation_enabled",
            desc_params={},
            method=method,
            method_calls=len(method_supplies.selected_nodes),
            graph_shard=shard,
        ),
    )


@atatus.capture_span()
def k8s_root_filesystem_read_only(
    shard: GraphShard, method_supplies: MethodSupplies
) -> MethodExecutionResult:
    method = MethodsEnum.K8S_ROOT_FILESYSTEM_READ_ONLY

    def n_ids() -> Iterator[NId]:
        graph = shard.syntax_graph
        integrity = check_template_integrity_for_all_nodes(
            graph, method_supplies.selected_nodes
        )
        if integrity:
            for nid in method_supplies.selected_nodes:
                for c_id in iterate_containers(graph, nid):
                    if report := _root_filesystem_read_only(graph, c_id):
                        yield report

    return get_vulnerabilities_from_n_ids(
        n_ids=n_ids(),
        query_supplies=QuerySupplies(
            desc_key="f267.k8s_root_filesystem_read_only",
            desc_params={},
            method=method,
            method_calls=len(method_supplies.selected_nodes),
            graph_shard=shard,
        ),
    )


@atatus.capture_span()
def k8s_check_seccomp_profile(
    shard: GraphShard, method_supplies: MethodSupplies
) -> MethodExecutionResult:
    method = MethodsEnum.K8S_CHECK_SECCOMP_PROFILE

    def n_ids() -> Iterator[NId]:
        graph = shard.syntax_graph
        integrity = check_template_integrity_for_all_nodes(
            graph, method_supplies.selected_nodes
        )
        if integrity:
            for nid in method_supplies.selected_nodes:
                for report in _check_seccomp_profile(graph, nid):
                    yield report

    return get_vulnerabilities_from_n_ids(
        n_ids=n_ids(),
        query_supplies=QuerySupplies(
            desc_key="f267.k8s_check_seccomp_profile",
            desc_params={},
            method=method,
            method_calls=len(method_supplies.selected_nodes),
            graph_shard=shard,
        ),
    )


@atatus.capture_span()
def k8s_check_add_capability(
    shard: GraphShard, method_supplies: MethodSupplies
) -> MethodExecutionResult:
    method = MethodsEnum.K8S_CHECK_ADD_CAPABILITY

    def n_ids() -> Iterator[NId]:
        graph = shard.syntax_graph
        integrity = check_template_integrity_for_all_nodes(
            graph, method_supplies.selected_nodes
        )
        if integrity:
            for nid in method_supplies.selected_nodes:
                for sc_id in iterate_security_context(graph, nid, True):
                    for report in _check_add_capability(graph, sc_id):
                        yield report

    return get_vulnerabilities_from_n_ids(
        n_ids=n_ids(),
        query_supplies=QuerySupplies(
            desc_key="f267.k8s_check_add_capability",
            desc_params={},
            method=method,
            method_calls=len(method_supplies.selected_nodes),
            graph_shard=shard,
        ),
    )


@atatus.capture_span()
def k8s_check_run_as_user(
    shard: GraphShard, method_supplies: MethodSupplies
) -> MethodExecutionResult:
    method = MethodsEnum.K8S_CHECK_RUN_AS_USER

    def n_ids() -> Iterator[NId]:
        graph = shard.syntax_graph
        integrity = check_template_integrity_for_all_nodes(
            graph, method_supplies.selected_nodes
        )
        if integrity:
            for nid in method_supplies.selected_nodes:
                for sc_id in iterate_security_context(graph, nid, False):
                    for report in _check_run_as_user(graph, sc_id):
                        yield report

    return get_vulnerabilities_from_n_ids(
        n_ids=n_ids(),
        query_supplies=QuerySupplies(
            desc_key="f267.k8s_check_run_as_user",
            desc_params={},
            method=method,
            method_calls=len(method_supplies.selected_nodes),
            graph_shard=shard,
        ),
    )


@atatus.capture_span()
def k8s_check_privileged_used(
    shard: GraphShard, method_supplies: MethodSupplies
) -> MethodExecutionResult:
    method = MethodsEnum.K8S_CHECK_PRIVILEGED_USED

    def n_ids() -> Iterator[NId]:
        graph = shard.syntax_graph
        integrity = check_template_integrity_for_all_nodes(
            graph, method_supplies.selected_nodes
        )
        if integrity:
            for nid in method_supplies.selected_nodes:
                for sc_id in iterate_security_context(graph, nid, True):
                    for report in _check_privileged_used(graph, sc_id):
                        yield report

    return get_vulnerabilities_from_n_ids(
        n_ids=n_ids(),
        query_supplies=QuerySupplies(
            desc_key="f267.k8s_check_privileged_used",
            desc_params={},
            method=method,
            method_calls=len(method_supplies.selected_nodes),
            graph_shard=shard,
        ),
    )


@atatus.capture_span()
def k8s_check_drop_capability(
    shard: GraphShard, method_supplies: MethodSupplies
) -> MethodExecutionResult:
    method = MethodsEnum.K8S_CHECK_DROP_CAPABILITY

    def n_ids() -> Iterator[NId]:
        graph = shard.syntax_graph
        integrity = check_template_integrity_for_all_nodes(
            graph, method_supplies.selected_nodes
        )
        if integrity:
            for nid in method_supplies.selected_nodes:
                for sc_id in iterate_security_context(graph, nid, True):
                    for report in _check_drop_capability(graph, sc_id):
                        yield report

    return get_vulnerabilities_from_n_ids(
        n_ids=n_ids(),
        query_supplies=QuerySupplies(
            desc_key="f267.k8s_check_drop_capability",
            desc_params={},
            method=method,
            method_calls=len(method_supplies.selected_nodes),
            graph_shard=shard,
        ),
    )


@atatus.capture_span()
def k8s_check_if_capability_exists(
    shard: GraphShard, method_supplies: MethodSupplies
) -> MethodExecutionResult:
    method = MethodsEnum.K8S_CHECK_IF_CAPABILITY_EXISTS

    def n_ids() -> Iterator[NId]:
        graph = shard.syntax_graph
        integrity = check_template_integrity_for_all_nodes(
            graph, method_supplies.selected_nodes
        )
        if integrity:
            for nid in method_supplies.selected_nodes:
                for sc_id in iterate_security_context(graph, nid, True):
                    for report in _check_if_capability_exists(graph, sc_id):
                        yield report

    return get_vulnerabilities_from_n_ids(
        n_ids=n_ids(),
        query_supplies=QuerySupplies(
            desc_key="f267.k8s_check_if_capability_exists",
            desc_params={},
            method=method,
            method_calls=len(method_supplies.selected_nodes),
            graph_shard=shard,
        ),
    )


@atatus.capture_span()
def k8s_check_if_sys_admin_exists(
    shard: GraphShard, method_supplies: MethodSupplies
) -> MethodExecutionResult:
    method = MethodsEnum.K8S_CHECK_IF_SYS_ADMIN_EXISTS

    def n_ids() -> Iterator[NId]:
        graph = shard.syntax_graph
        integrity = check_template_integrity_for_all_nodes(
            graph, method_supplies.selected_nodes
        )
        if integrity:
            for nid in method_supplies.selected_nodes:
                for sc_id in iterate_security_context(graph, nid, True):
                    for report in _check_if_sys_admin_exists(graph, sc_id):
                        yield report

    return get_vulnerabilities_from_n_ids(
        n_ids=n_ids(),
        query_supplies=QuerySupplies(
            desc_key="f267.k8s_check_if_sys_admin_exists",
            desc_params={},
            method=method,
            method_calls=len(method_supplies.selected_nodes),
            graph_shard=shard,
        ),
    )


@atatus.capture_span()
def k8s_container_without_securitycontext(
    shard: GraphShard, method_supplies: MethodSupplies
) -> MethodExecutionResult:
    method = MethodsEnum.K8S_CONTAINER_WITHOUT_SECURITYCONTEXT

    def n_ids() -> Iterator[NId]:
        graph = shard.syntax_graph
        integrity = check_template_integrity_for_all_nodes(
            graph, method_supplies.selected_nodes
        )
        if integrity:
            for nid in method_supplies.selected_nodes:
                for report in _container_without_securitycontext(graph, nid):
                    yield report

    return get_vulnerabilities_from_n_ids(
        n_ids=n_ids(),
        query_supplies=QuerySupplies(
            desc_key="f267.k8s_container_without_securitycontext",
            desc_params={},
            method=method,
            method_calls=len(method_supplies.selected_nodes),
            graph_shard=shard,
        ),
    )


@atatus.capture_span()
def k8s_check_host_pid(
    shard: GraphShard, method_supplies: MethodSupplies
) -> MethodExecutionResult:
    method = MethodsEnum.K8S_CHECK_HOST_PID

    def n_ids() -> Iterator[NId]:
        graph = shard.syntax_graph
        integrity = check_template_integrity_for_all_nodes(
            graph, method_supplies.selected_nodes
        )
        if integrity:
            for nid in method_supplies.selected_nodes:
                for c_id in get_host_pid(graph, nid):
                    yield c_id

    return get_vulnerabilities_from_n_ids(
        n_ids=n_ids(),
        query_supplies=QuerySupplies(
            desc_key="f267.k8s_check_host_pid",
            desc_params={},
            method=method,
            method_calls=len(method_supplies.selected_nodes),
            graph_shard=shard,
        ),
    )
