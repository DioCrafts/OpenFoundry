import { OpenFoundryClient } from '../index';

async function main() {
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

  const overview = {
    datasets: datasets.datasets?.length ?? 0,
    ontologyTypes: ontology.object_types?.length ?? 0,
    pipelines: pipelines.pipelines?.length ?? 0,
    maintenanceMode: controlPanel.maintenance_mode ?? false,
    deploymentMode: controlPanel.deployment_mode ?? 'unknown',
  };

  console.log('Phase 3 control-plane overview:', overview);
}

void main();
