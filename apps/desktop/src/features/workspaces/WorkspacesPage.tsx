import { useCallback, useEffect, useState } from 'react';
import { FolderOpen, Loader2, Plus, Trash2 } from 'lucide-react';
import { Button, Card, CardContent, CardHeader, CardTitle, useToast, ToastContainer } from '@mcpmux/ui';
import {
  listWorkspaceBindingsForSpace,
  createWorkspaceBinding,
  deleteWorkspaceBinding,
  type WorkspaceBinding,
} from '@/lib/api/workspaceBindings';
import { listFeatureSetsBySpace, type FeatureSet } from '@/lib/api/featureSets';
import { useViewSpace } from '@/stores';

/**
 * Workspaces page — CRUD for WorkspaceBinding (resolver v2, middle tier).
 *
 * A binding maps a normalized workspace root path to a FeatureSet. When an
 * MCP client reports roots via the MCP `roots` capability, the gateway's
 * FeatureSetResolver matches the longest-prefix binding for the client's
 * Space and uses that FS — unless the client has an explicit pin, which
 * always wins.
 */
export function WorkspacesPage() {
  const viewSpace = useViewSpace();
  const [bindings, setBindings] = useState<WorkspaceBinding[]>([]);
  const [featureSets, setFeatureSets] = useState<FeatureSet[]>([]);
  const [isLoading, setIsLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const { toasts, success, error: showError } = useToast();

  // Create form state
  const [showForm, setShowForm] = useState(false);
  const [isCreating, setIsCreating] = useState(false);
  const [formRoot, setFormRoot] = useState('');
  const [formFeatureSetId, setFormFeatureSetId] = useState('');

  const loadData = useCallback(async (spaceId?: string) => {
    setIsLoading(true);
    setError(null);
    try {
      if (!spaceId) {
        setBindings([]);
        setFeatureSets([]);
        return;
      }
      const [b, fs] = await Promise.all([
        listWorkspaceBindingsForSpace(spaceId),
        listFeatureSetsBySpace(spaceId),
      ]);
      setBindings(b);
      setFeatureSets(fs);
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    } finally {
      setIsLoading(false);
    }
  }, []);

  useEffect(() => {
    loadData(viewSpace?.id);
  }, [viewSpace?.id, loadData]);

  const handleCreate = async () => {
    if (!viewSpace) return;
    if (!formRoot.trim() || !formFeatureSetId) {
      showError('Missing fields', 'Workspace root and FeatureSet are both required.');
      return;
    }
    setIsCreating(true);
    try {
      const created = await createWorkspaceBinding(
        viewSpace.id,
        formRoot.trim(),
        formFeatureSetId
      );
      setBindings((prev) => [...prev, created]);
      setFormRoot('');
      setFormFeatureSetId('');
      setShowForm(false);
      success('Binding created', created.workspace_root);
    } catch (e) {
      showError('Failed to create binding', e instanceof Error ? e.message : String(e));
    } finally {
      setIsCreating(false);
    }
  };

  const handleDelete = async (binding: WorkspaceBinding) => {
    try {
      await deleteWorkspaceBinding(binding.id);
      setBindings((prev) => prev.filter((b) => b.id !== binding.id));
      success('Binding removed', binding.workspace_root);
    } catch (e) {
      showError('Failed to remove binding', e instanceof Error ? e.message : String(e));
    }
  };

  const featureSetName = (id: string) =>
    featureSets.find((fs) => fs.id === id)?.name ?? id;

  if (!viewSpace) {
    return (
      <div className="p-8">
        <p className="text-[rgb(var(--muted))]">Select a Space to manage workspace bindings.</p>
      </div>
    );
  }

  return (
    <div className="p-8 max-w-5xl mx-auto">
      <ToastContainer
        toasts={toasts}
        onClose={(id) => toasts.find((t) => t.id === id)?.onClose(id)}
      />

      <div className="flex items-start justify-between mb-6">
        <div>
          <h1 className="text-2xl font-semibold flex items-center gap-2">
            <FolderOpen className="h-6 w-6" /> Workspace Bindings
          </h1>
          <p className="text-sm text-[rgb(var(--muted))] mt-1 max-w-2xl">
            Bind a workspace folder to a FeatureSet. When an MCP client reports this
            folder as one of its roots, the gateway uses the bound FeatureSet — unless
            the client's access key has an explicit pin, which always wins.
          </p>
        </div>
        <Button
          variant="primary"
          size="md"
          onClick={() => setShowForm((s) => !s)}
          data-testid="workspace-binding-create-toggle"
        >
          <Plus className="h-4 w-4 mr-1" /> New binding
        </Button>
      </div>

      {error && (
        <Card className="mb-4 border-red-500/50">
          <CardContent className="p-4 text-sm text-red-600 dark:text-red-400">{error}</CardContent>
        </Card>
      )}

      {showForm && (
        <Card className="mb-6">
          <CardHeader>
            <CardTitle className="text-base">New binding</CardTitle>
          </CardHeader>
          <CardContent className="space-y-3">
            <div>
              <label className="block text-xs font-medium mb-1">Workspace root</label>
              <input
                type="text"
                value={formRoot}
                onChange={(e) => setFormRoot(e.target.value)}
                placeholder="/home/me/projects/android-app or D:\\work\\api"
                className="w-full px-3 py-2 border border-[rgb(var(--border))] rounded bg-[rgb(var(--surface))] text-sm"
                data-testid="workspace-binding-root-input"
              />
              <p className="text-xs text-[rgb(var(--muted))] mt-1">
                Will be normalized: Windows drive letters are lowercased, trailing separators stripped,
                and <code>file://</code> URIs converted to paths.
              </p>
            </div>
            <div>
              <label className="block text-xs font-medium mb-1">FeatureSet</label>
              <select
                value={formFeatureSetId}
                onChange={(e) => setFormFeatureSetId(e.target.value)}
                className="w-full px-3 py-2 border border-[rgb(var(--border))] rounded bg-[rgb(var(--surface))] text-sm"
                data-testid="workspace-binding-fs-select"
              >
                <option value="">Select a FeatureSet…</option>
                {featureSets.map((fs) => (
                  <option key={fs.id} value={fs.id}>
                    {fs.name}
                  </option>
                ))}
              </select>
            </div>
            <div className="flex justify-end gap-2">
              <Button variant="secondary" size="sm" onClick={() => setShowForm(false)}>
                Cancel
              </Button>
              <Button
                variant="primary"
                size="sm"
                onClick={handleCreate}
                disabled={isCreating}
                data-testid="workspace-binding-create-submit"
              >
                {isCreating ? <Loader2 className="h-4 w-4 animate-spin" /> : 'Create'}
              </Button>
            </div>
          </CardContent>
        </Card>
      )}

      {isLoading ? (
        <div className="flex items-center justify-center py-16">
          <Loader2 className="h-6 w-6 animate-spin text-[rgb(var(--muted))]" />
        </div>
      ) : bindings.length === 0 ? (
        <Card>
          <CardContent className="flex flex-col items-center justify-center py-16 text-center">
            <FolderOpen className="h-10 w-10 text-[rgb(var(--muted))] mb-3" />
            <p className="font-medium">No workspace bindings yet</p>
            <p className="text-sm text-[rgb(var(--muted))] max-w-md mt-1">
              Without a binding, the resolver falls back to the Space's active FeatureSet for
              every roots-capable client.
            </p>
          </CardContent>
        </Card>
      ) : (
        <div className="space-y-2">
          {bindings.map((b) => (
            <Card
              key={b.id}
              className="p-4 flex items-center justify-between"
              data-testid={`workspace-binding-row-${b.id}`}
            >
              <div className="flex-1 min-w-0 mr-4">
                <p className="font-mono text-sm truncate">{b.workspace_root}</p>
                <p className="text-xs text-[rgb(var(--muted))] mt-0.5">
                  → {featureSetName(b.feature_set_id)}
                </p>
              </div>
              <Button
                variant="secondary"
                size="sm"
                onClick={() => handleDelete(b)}
                data-testid={`workspace-binding-delete-${b.id}`}
              >
                <Trash2 className="h-4 w-4" />
              </Button>
            </Card>
          ))}
        </div>
      )}
    </div>
  );
}
