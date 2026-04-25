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
import { OpenFoundryProvider, useOpenFoundry, useOpenFoundryQuery } from '@open-foundry/sdk/react';

function DatasetCount() {
  const client = useOpenFoundry();
  const datasets = useOpenFoundryQuery(() => client.datasetDatasetListdatasets(), [client]);
  return <div>{datasets.data?.datasets?.length ?? 0}</div>;
}

function App() {
  return (
    <OpenFoundryProvider options={{ baseUrl: 'https://platform.example.com' }}>
      <DatasetCount />
    </OpenFoundryProvider>
  );
}
```
