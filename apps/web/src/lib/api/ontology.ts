import api from './client';

export interface ObjectType {
  id: string;
  name: string;
  display_name: string;
  description: string;
  primary_key_property: string | null;
  icon: string | null;
  color: string | null;
  owner_id: string;
  created_at: string;
  updated_at: string;
}

export interface Property {
  id: string;
  object_type_id: string;
  name: string;
  display_name: string;
  description: string;
  property_type: string;
  required: boolean;
  unique_constraint: boolean;
  default_value: unknown;
  validation_rules: unknown;
  created_at: string;
  updated_at: string;
}

export interface LinkType {
  id: string;
  name: string;
  display_name: string;
  description: string;
  source_type_id: string;
  target_type_id: string;
  cardinality: string;
  owner_id: string;
  created_at: string;
  updated_at: string;
}

export interface ObjectInstance {
  id: string;
  object_type_id: string;
  properties: Record<string, unknown>;
  created_by: string;
  created_at: string;
  updated_at: string;
}

export interface LinkInstance {
  id: string;
  link_type_id: string;
  source_object_id: string;
  target_object_id: string;
  properties: Record<string, unknown> | null;
  created_by: string;
  created_at: string;
}

export type ActionOperationKind =
  | 'update_object'
  | 'create_link'
  | 'delete_object'
  | 'invoke_function'
  | 'invoke_webhook';

export interface ActionInputField {
  name: string;
  display_name?: string | null;
  description?: string | null;
  property_type: string;
  required: boolean;
  default_value?: unknown;
}

export interface ActionType {
  id: string;
  name: string;
  display_name: string;
  description: string;
  object_type_id: string;
  operation_kind: ActionOperationKind;
  input_schema: ActionInputField[];
  config: unknown;
  confirmation_required: boolean;
  permission_key: string | null;
  owner_id: string;
  created_at: string;
  updated_at: string;
}

export interface ValidateActionResponse {
  valid: boolean;
  errors: string[];
  preview: unknown;
}

export interface ExecuteActionResponse {
  action: ActionType;
  target_object_id: string | null;
  deleted: boolean;
  preview: unknown;
  object: unknown | null;
  link: unknown | null;
  result: unknown | null;
}

export interface CreateActionTypeBody {
  name: string;
  display_name?: string;
  description?: string;
  object_type_id: string;
  operation_kind: ActionOperationKind;
  input_schema?: ActionInputField[];
  config?: unknown;
  confirmation_required?: boolean;
  permission_key?: string;
}

export interface UpdateActionTypeBody {
  display_name?: string;
  description?: string;
  operation_kind?: ActionOperationKind;
  input_schema?: ActionInputField[];
  config?: unknown;
  confirmation_required?: boolean;
  permission_key?: string;
}

// Object Types
export function listObjectTypes(params?: { page?: number; per_page?: number; search?: string }) {
  const qs = new URLSearchParams();
  if (params?.page) qs.set('page', String(params.page));
  if (params?.per_page) qs.set('per_page', String(params.per_page));
  if (params?.search) qs.set('search', params.search);
  return api.get<{ data: ObjectType[]; total: number; page: number; per_page: number }>(
    `/ontology/types?${qs}`,
  );
}

export function getObjectType(id: string) {
  return api.get<ObjectType>(`/ontology/types/${id}`);
}

export function createObjectType(body: {
  name: string;
  display_name?: string;
  description?: string;
  icon?: string;
  color?: string;
}) {
  return api.post<ObjectType>('/ontology/types', body);
}

export function updateObjectType(id: string, body: {
  display_name?: string;
  description?: string;
  icon?: string;
  color?: string;
}) {
  return api.put<ObjectType>(`/ontology/types/${id}`, body);
}

export function deleteObjectType(id: string) {
  return api.delete(`/ontology/types/${id}`);
}

// Action Types
export function listActionTypes(params?: {
  object_type_id?: string;
  page?: number;
  per_page?: number;
  search?: string;
}) {
  const qs = new URLSearchParams();
  if (params?.object_type_id) qs.set('object_type_id', params.object_type_id);
  if (params?.page) qs.set('page', String(params.page));
  if (params?.per_page) qs.set('per_page', String(params.per_page));
  if (params?.search) qs.set('search', params.search);
  return api.get<{ data: ActionType[]; total: number; page: number; per_page: number }>(
    `/ontology/actions?${qs}`,
  );
}

export function getActionType(id: string) {
  return api.get<ActionType>(`/ontology/actions/${id}`);
}

export function createActionType(body: CreateActionTypeBody) {
  return api.post<ActionType>('/ontology/actions', body);
}

export function updateActionType(id: string, body: UpdateActionTypeBody) {
  return api.put<ActionType>(`/ontology/actions/${id}`, body);
}

export function deleteActionType(id: string) {
  return api.delete(`/ontology/actions/${id}`);
}

export function validateAction(id: string, body: {
  target_object_id?: string;
  parameters?: Record<string, unknown>;
}) {
  return api.post<ValidateActionResponse>(`/ontology/actions/${id}/validate`, body);
}

export function executeAction(id: string, body: {
  target_object_id?: string;
  parameters?: Record<string, unknown>;
}) {
  return api.post<ExecuteActionResponse>(`/ontology/actions/${id}/execute`, body);
}

// Properties
export function listProperties(typeId: string) {
  return api.get<Property[]>(`/ontology/types/${typeId}/properties`);
}

export function createProperty(typeId: string, body: {
  name: string;
  display_name?: string;
  description?: string;
  property_type: string;
  required?: boolean;
  unique_constraint?: boolean;
}) {
  return api.post<Property>(`/ontology/types/${typeId}/properties`, body);
}

// Link Types
export function listLinkTypes(params?: { object_type_id?: string; page?: number; per_page?: number }) {
  const qs = new URLSearchParams();
  if (params?.object_type_id) qs.set('object_type_id', params.object_type_id);
  if (params?.page) qs.set('page', String(params.page));
  if (params?.per_page) qs.set('per_page', String(params.per_page));
  return api.get<{ data: LinkType[]; total: number }>(`/ontology/links?${qs}`);
}

export function createLinkType(body: {
  name: string;
  display_name?: string;
  description?: string;
  source_type_id: string;
  target_type_id: string;
  cardinality?: string;
}) {
  return api.post<LinkType>('/ontology/links', body);
}

export function deleteLinkType(id: string) {
  return api.delete(`/ontology/links/${id}`);
}

// Object Instances
export function listObjects(typeId: string, params?: { page?: number; per_page?: number }) {
  const qs = new URLSearchParams();
  if (params?.page) qs.set('page', String(params.page));
  if (params?.per_page) qs.set('per_page', String(params.per_page));
  return api.get<{ data: ObjectInstance[]; total: number }>(`/ontology/types/${typeId}/objects?${qs}`);
}

export function createObject(typeId: string, properties: Record<string, unknown>) {
  return api.post<ObjectInstance>(`/ontology/types/${typeId}/objects`, { properties });
}

export function deleteObject(typeId: string, objectId: string) {
  return api.delete(`/ontology/types/${typeId}/objects/${objectId}`);
}
