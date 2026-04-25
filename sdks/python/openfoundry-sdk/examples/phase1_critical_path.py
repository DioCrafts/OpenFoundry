from __future__ import annotations

from openfoundry_sdk import OpenFoundryClient


def build_client(base_url: str, token: str) -> OpenFoundryClient:
    return OpenFoundryClient(
        base_url=base_url,
        headers={"authorization": f"Bearer {token}"},
    )


def critical_path_calls(client: OpenFoundryClient) -> dict[str, object]:
    return {
        "datasets": client.dataset_dataset_listdatasets,
        "ontology_types": client.ontology_ontology_listobjecttypes,
        "pipelines": client.pipeline_pipeline_listpipelines,
        "control_panel": client.admin_v2_getcontrolpanel,
    }


if __name__ == "__main__":
    client = build_client("https://platform.openfoundry.local", "<token>")
    print(sorted(critical_path_calls(client).keys()))
