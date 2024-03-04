# code taken from:
# https://gitlab.com/fluidattacks/universe/-/blob/trunk/skims/skims/lib_sast/root/f267/kubernetes.py


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
        if integrity:  # 2 (nested = 1)
            for nid in method_supplies.selected_nodes:  # 3 (nested = 2)
                for c_id in get_host_pid(graph, nid):  # 4 (nested = 3)
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
