import { useState, useEffect, useCallback } from 'react';
import {
  Plus,
  Loader2,
  Server,
  Package,
  Settings,
  X,
  RefreshCw,
  Globe,
  Star,
  Search,
  AlertCircle,
  CheckCircle2,
  Zap,
} from 'lucide-react';
import {
  Card,
  CardHeader,
  CardTitle,
  CardContent,
  Button,
  useToast,
  ToastContainer,
} from '@mcpmux/ui';
import type { FeatureSet, CreateFeatureSetInput } from '@/lib/api/featureSets';
import {
  listFeatureSetsBySpace,
  createFeatureSet,
  deleteFeatureSet,
  getFeatureSetWithMembers,
} from '@/lib/api/featureSets';
import { setSpaceActiveFeatureSet, getSpace } from '@/lib/api/spaces';
import { useViewSpace } from '@/stores';
import { FeatureSetPanel } from './FeatureSetPanel';

// Get icon for feature set type
const getFeatureSetIcon = (fs: FeatureSet) => {
  if (fs.icon) return <span className="text-xl">{fs.icon}</span>;
  
  switch (fs.feature_set_type) {
    case 'all':
      return <Globe className="h-8 w-8 text-green-500" />;
    case 'default':
      return <Star className="h-8 w-8 text-yellow-500" />;
    case 'server-all':
      return <Server className="h-8 w-8 text-blue-500" />;
    case 'custom':
    default:
      return <Package className="h-8 w-8 text-purple-500" />;
  }
};

// Get display name for feature set type
const getFeatureSetTypeName = (type: string) => {
  switch (type) {
    case 'all':
      return 'All Features';
    case 'default':
      return 'Default';
    case 'server-all':
      return 'Server All';
    case 'custom':
    default:
      return 'Custom';
  }
};

export function FeatureSetsPage() {
  const [featureSets, setFeatureSets] = useState<FeatureSet[]>([]);
  const viewSpace = useViewSpace();
  const [isLoading, setIsLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [searchQuery, setSearchQuery] = useState('');
  const { toasts, success, error: showError } = useToast();
  
  // Create modal state
  const [showCreateModal, setShowCreateModal] = useState(false);
  const [isCreating, setIsCreating] = useState(false);
  const [createName, setCreateName] = useState('');
  const [createDescription, setCreateDescription] = useState('');
  const [createIcon, setCreateIcon] = useState('');
  
  // Panel state
  const [selectedFeatureSet, setSelectedFeatureSet] = useState<FeatureSet | null>(null);

  // Resolver v2: which FS is the Space's active fallback.
  // Tracked locally so the "Active" badge updates immediately after clicking
  // "Set Active" without waiting for a refetch of the whole viewSpace.
  const [activeFeatureSetId, setActiveFeatureSetId] = useState<string | null>(null);
  // Id of the FS whose "Set Active" button is mid-flight, so we can render
  // a spinner in its place (otherwise the optimistic update swaps the button
  // for the Active badge immediately and a slow backend feels like a no-op).
  const [activatingId, setActivatingId] = useState<string | null>(null);

  const loadData = useCallback(async (spaceId?: string) => {
    setIsLoading(true);
    setError(null);
    try {
      if (!spaceId) {
        setFeatureSets([]);
        return;
      }
      
      // Backend filters out server-all feature sets for disabled servers
      const data = await listFeatureSetsBySpace(spaceId);
      setFeatureSets(data);

      // Fetch the Space's active FS for the "Active" badge. The
      // viewSpace from the store may be stale, so we fetch fresh here.
      const space = await getSpace(spaceId);
      setActiveFeatureSetId(space?.active_feature_set_id ?? null);
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    } finally {
      setIsLoading(false);
    }
  }, []);

  const handleSetActive = async (fs: FeatureSet, event: React.MouseEvent) => {
    // Belt and suspenders: stop both the synthetic event AND the native
    // event so the wrapping Card's onClick can't open the panel.
    event.stopPropagation();
    event.preventDefault();
    event.nativeEvent?.stopImmediatePropagation?.();

    if (!viewSpace) return;
    if (activatingId) return; // Debounce double-clicks.

    const previous = activeFeatureSetId;
    setActivatingId(fs.id);
    setActiveFeatureSetId(fs.id); // Optimistic.
    try {
      await setSpaceActiveFeatureSet(viewSpace.id, fs.id);
      success(
        `${fs.name} is now Active`,
        'Applied to every connected client in this Space without a pin or workspace binding.'
      );
    } catch (e) {
      setActiveFeatureSetId(previous);
      const msg = e instanceof Error ? e.message : String(e);
      showError('Failed to set Active FeatureSet', msg);
    } finally {
      setActivatingId(null);
    }
  };

  useEffect(() => {
    setSelectedFeatureSet(null);
    setShowCreateModal(false);
    loadData(viewSpace?.id);
  }, [viewSpace?.id, loadData]);

  const handleCreate = async () => {
    if (!createName.trim() || !viewSpace) return;
    
    setIsCreating(true);
    setError(null);
    try {
      const input: CreateFeatureSetInput = {
        name: createName.trim(),
        space_id: viewSpace.id,
        description: createDescription.trim() || undefined,
        icon: createIcon.trim() || undefined,
      };
      const newFs = await createFeatureSet(input);
      setFeatureSets((prev) => [...prev, newFs]);
      setCreateName('');
      setCreateDescription('');
      setCreateIcon('');
      setShowCreateModal(false);
      
      success('Feature set created', `"${newFs.name}" has been created successfully`);
      
      // Automatically open the new feature set
      handleOpenPanel(newFs);
    } catch (e) {
      const errorMsg = e instanceof Error ? e.message : String(e);
      setError(errorMsg);
      showError('Failed to create feature set', errorMsg);
    } finally {
      setIsCreating(false);
    }
  };

  const handleDelete = async (id: string) => {
    // Confirmation handled by caller if needed, but we do it here too just in case called directly
    try {
      const deletedSet = featureSets.find(fs => fs.id === id);
      await deleteFeatureSet(id);
      setFeatureSets((prev) => prev.filter((fs) => fs.id !== id));
      if (selectedFeatureSet?.id === id) {
        setSelectedFeatureSet(null);
      }
      
      success('Feature set deleted', `"${deletedSet?.name || 'Feature set'}" has been deleted`);
    } catch (e) {
      const errorMsg = e instanceof Error ? e.message : String(e);
      setError(errorMsg);
      showError('Failed to delete feature set', errorMsg);
    }
  };

  const handleOpenPanel = async (fs: FeatureSet) => {
    try {
      const fullFs = await getFeatureSetWithMembers(fs.id);
      if (fullFs) {
        setSelectedFeatureSet(fullFs);
      }
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    }
  };

  const handlePanelClose = () => {
    setSelectedFeatureSet(null);
    loadData(viewSpace?.id); // Refresh list to get updated member counts etc.
  };

  // Filter and sort feature sets (backend already filters server-all for disabled servers)
  const filteredSets = featureSets
    .filter(fs => {
      // Hide implicit custom sets
      if (fs.name.endsWith(' - Custom')) return false;

      // Apply search filter
      if (!searchQuery) return true;
      const query = searchQuery.toLowerCase();
      return (
        fs.name.toLowerCase().includes(query) ||
        fs.description?.toLowerCase().includes(query) ||
        fs.feature_set_type.toLowerCase().includes(query)
      );
    })
    .sort((a, b) => {
      // Sort order: all → default → custom → server-all
      const order: Record<string, number> = { all: 0, default: 1, custom: 2, 'server-all': 3 };
      const aOrder = order[a.feature_set_type] ?? 2;
      const bOrder = order[b.feature_set_type] ?? 2;
      return aOrder - bOrder;
    });

  return (
    <>
      <ToastContainer toasts={toasts} onClose={(id) => toasts.find(t => t.id === id)?.onClose(id)} />
      <div className="h-full flex flex-col relative" data-testid="featuresets-page">
      {/* Header */}
      <div className="flex-shrink-0 p-8 border-b border-[rgb(var(--border-subtle))]">
        <div className="max-w-[2000px] mx-auto">
          <div className="flex flex-col sm:flex-row sm:items-start sm:justify-between gap-4 mb-6">
            <div className="flex-1 min-w-0">
              <div className="flex flex-wrap items-center gap-3 mb-2">
                <h1 className="text-3xl font-bold">Feature Sets</h1>
                {viewSpace && (
                  <span className="px-2 py-0.5 rounded-full bg-[rgb(var(--surface-elevated))] text-xs border border-[rgb(var(--border))] whitespace-nowrap">
                    {viewSpace.icon || '📁'} {viewSpace.name}
                  </span>
                )}
              </div>
              <p className="text-base text-[rgb(var(--muted))]">
                Manage reusable collections of features, prompts, and resources
              </p>
            </div>
            <div className="flex gap-3 flex-shrink-0">
              <Button 
                variant="ghost" 
                size="md" 
                onClick={() => loadData(viewSpace?.id)}
                disabled={isLoading}
              >
                <RefreshCw className={`h-4 w-4 mr-2 ${isLoading ? 'animate-spin' : ''}`} />
                Refresh
              </Button>
              <Button variant="primary" size="md" onClick={() => setShowCreateModal(true)}>
                <Plus className="h-4 w-4 mr-2" />
                Create Feature Set
              </Button>
            </div>
          </div>

          {/* Search Bar */}
          <div className="relative max-w-3xl">
            <Search className="absolute left-4 top-1/2 -translate-y-1/2 h-5 w-5 text-[rgb(var(--muted))]" />
            <input
              type="text"
              placeholder="Search feature sets..."
              value={searchQuery}
              onChange={(e) => setSearchQuery(e.target.value)}
              className="w-full pl-12 pr-4 py-3 text-base bg-[rgb(var(--surface))] border border-[rgb(var(--border))] rounded-xl focus:outline-none focus:ring-2 focus:ring-primary-500 focus:border-primary-500 transition-all"
            />
          </div>
        </div>
      </div>

      {/* Active FeatureSet explainer — helps users understand what the green
          ribbon / "Set Active" button actually does before they click around. */}
      <div className="flex-shrink-0 px-8 pt-6">
        <div className="max-w-[2000px] mx-auto flex items-start gap-3 p-4 rounded-xl border border-emerald-200/70 dark:border-emerald-800/40 bg-gradient-to-r from-emerald-50/60 to-transparent dark:from-emerald-900/15">
          <div className="mt-0.5 flex h-8 w-8 items-center justify-center rounded-lg bg-gradient-to-br from-emerald-500 to-green-500 text-white shadow-[0_4px_10px_-2px_rgb(16_185_129/0.45)]">
            <Zap className="h-4 w-4 fill-current" />
          </div>
          <div className="flex-1 min-w-0">
            <p className="text-sm font-semibold text-[rgb(var(--foreground))]">
              One <span className="text-emerald-600 dark:text-emerald-400">Active</span> FeatureSet per Space
            </p>
            <p className="text-xs text-[rgb(var(--muted))] mt-0.5 leading-relaxed">
              The Active set is applied to every connected MCP client in this Space that doesn&apos;t
              have an explicit pin or a matching workspace binding. Click <span className="font-medium text-emerald-700 dark:text-emerald-300">Set Active</span> on any card to make it the default.
            </p>
          </div>
        </div>
      </div>

      {/* Error */}
      {error && (
        <div className="flex-shrink-0 px-8 pt-6">
          <div className="max-w-[2000px] mx-auto p-4 bg-red-50 dark:bg-red-900/20 border border-red-200 dark:border-red-800 rounded-xl flex items-start gap-3">
            <AlertCircle className="h-5 w-5 text-red-600 dark:text-red-400 flex-shrink-0 mt-0.5" />
            <p className="text-base text-red-600 dark:text-red-400">{error}</p>
          </div>
        </div>
      )}

      {/* Content Grid */}
      <div className="flex-1 overflow-auto px-8 py-8">
        <div className="max-w-[2000px] mx-auto">
          {isLoading ? (
            <div className="flex items-center justify-center h-64">
              <Loader2 className="h-8 w-8 animate-spin text-primary-500" />
            </div>
          ) : filteredSets.length === 0 ? (
            <Card className="max-w-2xl mx-auto">
              <CardContent className="flex flex-col items-center justify-center py-16">
                <Package className="h-16 w-16 text-[rgb(var(--muted))] mb-4" />
                <h3 className="text-lg font-medium mb-2">
                  {searchQuery ? 'No feature sets match your search' : 'No feature sets created'}
                </h3>
                <p className="text-sm text-[rgb(var(--muted))] text-center max-w-md mb-6">
                  {searchQuery 
                    ? 'Try adjusting your search terms' 
                    : 'Create a feature set to group tools and resources together for easy access control.'
                  }
                </p>
                {!searchQuery && (
                  <Button variant="primary" onClick={() => setShowCreateModal(true)}>
                    <Plus className="h-4 w-4 mr-2" />
                    Create Feature Set
                  </Button>
                )}
              </CardContent>
            </Card>
          ) : (
            <div className="grid gap-5 auto-fill-cards">
              {filteredSets.map((fs) => {
                const isSelected = selectedFeatureSet?.id === fs.id;
                const isBuiltin = fs.is_builtin;
                const isActive = activeFeatureSetId === fs.id;

                const isActivating = activatingId === fs.id;

                return (
                  <Card
                    key={fs.id}
                    className={`relative overflow-hidden cursor-pointer transition-all duration-300 hover:shadow-lg hover:scale-[1.01] ${
                      isSelected ? 'ring-2 ring-primary-500 shadow-lg' : ''
                    } ${
                      isActive
                        ? 'ring-2 ring-emerald-400 shadow-[0_0_0_4px_rgb(16_185_129/0.08),0_12px_24px_-8px_rgb(16_185_129/0.35)] bg-gradient-to-br from-emerald-50/60 via-transparent to-transparent dark:from-emerald-900/20 dark:via-transparent dark:to-transparent'
                        : ''
                    }`}
                    onClick={() => handleOpenPanel(fs)}
                    data-testid={`featureset-card-${fs.id}`}
                  >
                    {/* Active ribbon — top-right corner badge, premium feel */}
                    {isActive && (
                      <div
                        className="absolute top-3 right-3 flex items-center gap-1.5 px-2.5 py-1 rounded-full bg-gradient-to-r from-emerald-500 to-green-500 text-white text-[10px] font-bold uppercase tracking-wider shadow-[0_4px_12px_-2px_rgb(16_185_129/0.5)]"
                        title="Applied to every client in this Space with no pin or workspace binding"
                        data-testid={`featureset-active-badge-${fs.id}`}
                      >
                        <Zap className="h-3 w-3 fill-current" />
                        Active
                      </div>
                    )}

                    <CardContent className="p-6">
                      {/* Header */}
                      <div className="flex items-start gap-4 mb-5">
                        <div
                          className={`w-16 h-16 flex items-center justify-center rounded-xl flex-shrink-0 border transition-colors ${
                            isActive
                              ? 'bg-emerald-50 dark:bg-emerald-900/30 border-emerald-200 dark:border-emerald-800/60'
                              : 'bg-[rgb(var(--surface))] border-[rgb(var(--border-subtle))]'
                          }`}
                        >
                          {getFeatureSetIcon(fs)}
                        </div>
                        <div className="flex-1 min-w-0 pr-16">
                          <h3 className="font-semibold text-lg truncate mb-1.5">{fs.name}</h3>
                          <span
                            className={`inline-flex items-center px-2.5 py-0.5 rounded-full text-xs font-medium ${
                              isBuiltin
                                ? 'bg-blue-100 dark:bg-blue-900/30 text-blue-700 dark:text-blue-300'
                                : 'bg-purple-100 dark:bg-purple-900/30 text-purple-700 dark:text-purple-300'
                            }`}
                          >
                            {getFeatureSetTypeName(fs.feature_set_type)}
                          </span>
                        </div>
                      </div>

                      {/* Description */}
                      <p className="text-sm text-[rgb(var(--muted))] line-clamp-2 mb-4 h-10">
                        {fs.description || 'No description provided.'}
                      </p>

                      {/* Footer — Set Active is now a prominent gradient button; Active state gets its own caption */}
                      <div className="flex items-center justify-between gap-3 text-xs text-[rgb(var(--muted))] border-t border-[rgb(var(--border-subtle))] pt-4">
                        <div className="flex items-center gap-1.5 flex-shrink-0">
                          {fs.feature_set_type === 'server-all' ? (
                            <span className="truncate max-w-[150px]">{fs.server_id}</span>
                          ) : fs.feature_set_type === 'all' ? (
                            <span className="italic">All features</span>
                          ) : (
                            <span>{fs.members?.length || 0} members</span>
                          )}
                        </div>

                        <div className="flex items-center gap-2 min-w-0">
                          {isActive ? (
                            <span
                              className="inline-flex items-center gap-1.5 text-emerald-600 dark:text-emerald-400 font-medium text-xs"
                              title="This FeatureSet is the Space's fallback"
                            >
                              <CheckCircle2 className="h-3.5 w-3.5" />
                              Applied to this Space
                            </span>
                          ) : (
                            <button
                              type="button"
                              onClick={(e) => handleSetActive(fs, e)}
                              onMouseDown={(e) => e.stopPropagation()}
                              disabled={isActivating}
                              className="group relative inline-flex items-center gap-1.5 px-3 py-1.5 rounded-full text-[11px] font-semibold uppercase tracking-wide text-emerald-700 dark:text-emerald-300 bg-emerald-50 dark:bg-emerald-900/20 border border-emerald-200 dark:border-emerald-800/60 hover:text-white hover:bg-gradient-to-r hover:from-emerald-500 hover:to-green-500 hover:border-transparent hover:shadow-[0_6px_16px_-4px_rgb(16_185_129/0.5)] disabled:opacity-60 disabled:cursor-wait transition-all duration-200"
                              title="Make this the default FeatureSet for every connected client in this Space"
                              data-testid={`featureset-set-active-${fs.id}`}
                            >
                              {isActivating ? (
                                <Loader2 className="h-3 w-3 animate-spin" />
                              ) : (
                                <Zap className="h-3 w-3 group-hover:fill-current transition-all" />
                              )}
                              {isActivating ? 'Activating…' : 'Set Active'}
                            </button>
                          )}
                          {isBuiltin && fs.feature_set_type !== 'default' ? (
                            <span className="italic flex-shrink-0">Auto-managed</span>
                          ) : (
                            <span className="hidden md:flex items-center gap-1 hover:text-primary-500 transition-colors flex-shrink-0">
                              Configure <Settings className="h-3 w-3" />
                            </span>
                          )}
                        </div>
                      </div>
                    </CardContent>
                  </Card>
                );
              })}
            </div>
          )}
        </div>
      </div>

      {/* Overlay backdrop when panel is open */}
      {selectedFeatureSet && (
        <div 
          data-testid="featureset-panel-overlay"
          className="fixed inset-0 bg-black/20 backdrop-blur-[2px] z-40 animate-in fade-in duration-200"
          onClick={() => setSelectedFeatureSet(null)}
        />
      )}

      {/* Slide-out Panel */}
      {selectedFeatureSet && viewSpace && (
        <FeatureSetPanel
          featureSet={selectedFeatureSet}
          spaceId={viewSpace.id}
          onClose={handlePanelClose}
          onDelete={handleDelete}
          onUpdate={() => loadData(viewSpace.id)}
        />
      )}

      {/* Create Modal */}
      {showCreateModal && (
        <div className="fixed inset-0 bg-black/50 flex items-center justify-center z-50">
          <Card className="w-full max-w-md mx-4 animate-in fade-in zoom-in-95 duration-200">
            <CardHeader>
              <CardTitle className="flex items-center justify-between">
                <span className="flex items-center gap-2">
                  <Plus className="h-5 w-5" />
                  Create Feature Set
                </span>
                <button
                  onClick={() => setShowCreateModal(false)}
                  className="p-1 rounded hover:bg-[rgb(var(--surface-hover))]"
                >
                  <X className="h-4 w-4" />
                </button>
              </CardTitle>
            </CardHeader>
            <CardContent className="space-y-4">
              <div>
                <label className="block text-sm font-medium mb-1">Name *</label>
                <input
                  type="text"
                  value={createName}
                  onChange={(e) => setCreateName(e.target.value)}
                  placeholder="e.g., GitHub Read Only"
                  className="w-full px-3 py-2 rounded-lg border border-[rgb(var(--border))] bg-[rgb(var(--surface))] focus:outline-none focus:ring-2 focus:ring-primary-500"
                  autoFocus
                />
              </div>
              
              <div>
                <label className="block text-sm font-medium mb-1">Description</label>
                <input
                  type="text"
                  value={createDescription}
                  onChange={(e) => setCreateDescription(e.target.value)}
                  placeholder="What this feature set allows..."
                  className="w-full px-3 py-2 rounded-lg border border-[rgb(var(--border))] bg-[rgb(var(--surface))] focus:outline-none focus:ring-2 focus:ring-primary-500"
                />
              </div>
              
              <div>
                <label className="block text-sm font-medium mb-1">Icon (emoji)</label>
                <input
                  type="text"
                  value={createIcon}
                  onChange={(e) => setCreateIcon(e.target.value)}
                  placeholder="🔧"
                  className="w-full px-3 py-2 rounded-lg border border-[rgb(var(--border))] bg-[rgb(var(--surface))] focus:outline-none focus:ring-2 focus:ring-primary-500"
                  maxLength={2}
                />
              </div>
              
              <div className="flex gap-3 pt-2">
                <Button variant="ghost" onClick={() => setShowCreateModal(false)}>
                  Cancel
                </Button>
                <Button
                  variant="primary"
                  onClick={handleCreate}
                  disabled={isCreating || !createName.trim()}
                >
                  {isCreating ? <Loader2 className="h-4 w-4 animate-spin" /> : 'Create'}
                </Button>
              </div>
            </CardContent>
          </Card>
        </div>
      )}
    </div>
    </>
  );
}

export default FeatureSetsPage;
