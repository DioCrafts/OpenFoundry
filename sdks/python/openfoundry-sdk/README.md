# OpenFoundry Python SDK

Generated from `apps/web/static/generated/openapi/openfoundry.json`.

Version: `0.1.0`

## Usage

```python
from openfoundry_sdk import OpenFoundryClient

client = OpenFoundryClient(
    base_url="https://platform.example.com",
    headers={"authorization": "Bearer <token>"},
)
```

## Phase 1 critical path

```python
from openfoundry_sdk import OpenFoundryClient

client = OpenFoundryClient(
    base_url="https://platform.example.com",
    headers={"authorization": "Bearer <token>"},
)

paths = {
    "datasets": client.dataset_dataset_listdatasets,
    "ontology_types": client.ontology_ontology_listobjecttypes,
    "pipelines": client.pipeline_pipeline_listpipelines,
    "control_panel": client.admin_v2_getcontrolpanel,
}

print(sorted(paths.keys()))
```
