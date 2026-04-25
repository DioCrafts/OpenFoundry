import { OpenFoundryClient, type ControlPanelSettings } from '../index';

export async function phase1CriticalPath(client: OpenFoundryClient): Promise<{
	datasets: number;
	ontologyTypes: number;
	pipelines: number;
	controlPanel: ControlPanelSettings;
}> {
	const [datasets, ontology, pipelines, controlPanel] = await Promise.all([
		client.datasetDatasetListdatasets(),
		client.ontologyOntologyListobjecttypes(),
		client.pipelinePipelineListpipelines(),
		client.adminV2Getcontrolpanel()
	]);

	return {
		datasets: datasets.datasets?.length ?? 0,
		ontologyTypes: ontology.object_types?.length ?? 0,
		pipelines: pipelines.pipelines?.length ?? 0,
		controlPanel
	};
}
