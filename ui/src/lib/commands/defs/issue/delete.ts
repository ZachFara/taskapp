import { defineCommand } from '../../registry';
import { archive } from './_issue';

// There is no hard delete: this is Archive under the name people look for.
defineCommand({ id: 'issue.delete', label: 'Delete (archives)', icon: 'archive', group: 'Issue', shortcut: 'delete', run: archive });
