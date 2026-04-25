# OpenFoundry TypeScript SDK

Generated from `apps/web/static/generated/openapi/openfoundry.json`.

Version: `0.1.0`

## Usage

```ts
import { OpenFoundryClient } from '@open-foundry/sdk';

const client = new OpenFoundryClient({
  baseUrl: 'https://platform.example.com',
  headers: { authorization: 'Bearer <token>' },
});

const me = await client.authAuthGetMe();
```

## React helpers

```ts
import { OpenFoundryProvider, useDatasets, usePlatformOverview } from '@open-foundry/sdk/react';

function DatasetCount() {
  const datasets = useDatasets();
  return <div>{datasets.data?.datasets?.length ?? 0}</div>;
}

function Overview() {
  const overview = usePlatformOverview();
  return (
    <div>
      {overview.data?.datasets.datasets?.length ?? 0} datasets ·
      {overview.data?.ontologyTypes.object_types?.length ?? 0} ontology types
    </div>
  );
}

function App() {
  return (
    <OpenFoundryProvider options={{ baseUrl: 'https://platform.example.com' }}>
      <DatasetCount />
      <Overview />
    </OpenFoundryProvider>
  );
}
```

## Phase 1 critical path

```ts
import { OpenFoundryClient } from '@open-foundry/sdk';

const client = new OpenFoundryClient({
  baseUrl: 'https://platform.example.com',
  headers: { authorization: 'Bearer <token>' },
});

const [datasets, ontology, pipelines, controlPanel] = await Promise.all([
  client.datasetDatasetListdatasets(),
  client.ontologyOntologyListobjecttypes(),
  client.pipelinePipelineListpipelines(),
  client.adminV2Getcontrolpanel(),
]);

const summary = {
  datasets: datasets.datasets?.length ?? 0,
  ontologyTypes: ontology.object_types?.length ?? 0,
  pipelines: pipelines.pipelines?.length ?? 0,
  controlPanel,
};

console.log(summary.datasets, summary.ontologyTypes, summary.pipelines);
```

## Phase 3 control plane example

```ts
import './src/examples/phase3-control-plane';
```
