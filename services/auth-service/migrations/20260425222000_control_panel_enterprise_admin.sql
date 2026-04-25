ALTER TABLE control_panel_settings
    ADD COLUMN IF NOT EXISTS identity_provider_mappings JSONB NOT NULL DEFAULT '[]'::jsonb,
    ADD COLUMN IF NOT EXISTS resource_management_policies JSONB NOT NULL DEFAULT '[]'::jsonb,
    ADD COLUMN IF NOT EXISTS upgrade_assistant JSONB NOT NULL DEFAULT jsonb_build_object(
        'current_version', '2026.04.0',
        'target_version', '2026.05.0',
        'maintenance_window', 'Sun 02:00-04:00 UTC',
        'rollback_channel', 'stable',
        'preflight_checks', jsonb_build_array(
            jsonb_build_object(
                'id', 'backups',
                'label', 'Backups verified',
                'owner', 'platform',
                'status', 'pending',
                'notes', 'Confirm latest database and object-store snapshots before rollout.'
            ),
            jsonb_build_object(
                'id', 'sso',
                'label', 'Identity providers aligned',
                'owner', 'identity',
                'status', 'pending',
                'notes', 'Validate SAML/OIDC attribute mapping and org assignments.'
            ),
            jsonb_build_object(
                'id', 'sessions',
                'label', 'Guest/scoped sessions reviewed',
                'owner', 'security',
                'status', 'pending',
                'notes', 'Review long-lived sessions and revoke stale temporary access.'
            )
        ),
        'rollout_stages', jsonb_build_array(
            jsonb_build_object(
                'id', 'staging',
                'label', 'Stage staging',
                'rollout_percentage', 10,
                'status', 'pending'
            ),
            jsonb_build_object(
                'id', 'canary',
                'label', 'Promote canary tenants',
                'rollout_percentage', 30,
                'status', 'pending'
            ),
            jsonb_build_object(
                'id', 'full',
                'label', 'Full production rollout',
                'rollout_percentage', 100,
                'status', 'pending'
            )
        ),
        'rollback_steps', jsonb_build_array(
            'freeze write traffic',
            'restore previous release channel',
            're-enable revoked operations only after health checks return green'
        )
    );

UPDATE control_panel_settings
SET identity_provider_mappings = CASE
        WHEN jsonb_typeof(identity_provider_mappings) = 'array'
            AND jsonb_array_length(identity_provider_mappings) > 0 THEN identity_provider_mappings
        ELSE jsonb_build_array(
            jsonb_build_object(
                'provider_slug', 'enterprise-saml',
                'default_organization_id', NULL,
                'organization_claim', 'organization_id',
                'workspace_claim', 'workspace',
                'default_workspace', 'shared-enterprise',
                'classification_clearance_claim', 'classification_clearance',
                'default_classification_clearance', 'internal',
                'role_claim', 'roles',
                'default_roles', jsonb_build_array('viewer'),
                'allowed_email_domains', jsonb_build_array('example.com', 'openfoundry.dev')
            )
        )
    END,
    resource_management_policies = CASE
        WHEN jsonb_typeof(resource_management_policies) = 'array'
            AND jsonb_array_length(resource_management_policies) > 0 THEN resource_management_policies
        ELSE jsonb_build_array(
            jsonb_build_object(
                'name', 'enterprise-default',
                'tenant_tier', 'enterprise',
                'applies_to_org_ids', jsonb_build_array(),
                'applies_to_workspaces', jsonb_build_array(),
                'quota', jsonb_build_object(
                    'max_query_limit', 10000,
                    'max_distributed_query_workers', 8,
                    'max_pipeline_workers', 8,
                    'max_request_body_bytes', 52428800,
                    'requests_per_minute', 5000,
                    'max_storage_gb', 500,
                    'max_shared_spaces', 25,
                    'max_guest_sessions', 50
                )
            ),
            jsonb_build_object(
                'name', 'partner-shared-workspace',
                'tenant_tier', 'team',
                'applies_to_org_ids', jsonb_build_array(),
                'applies_to_workspaces', jsonb_build_array('shared-enterprise', 'partner-portal'),
                'quota', jsonb_build_object(
                    'max_query_limit', 5000,
                    'max_distributed_query_workers', 4,
                    'max_pipeline_workers', 4,
                    'max_request_body_bytes', 20971520,
                    'requests_per_minute', 900,
                    'max_storage_gb', 120,
                    'max_shared_spaces', 8,
                    'max_guest_sessions', 20
                )
            )
        )
    END
WHERE singleton_id = TRUE;
