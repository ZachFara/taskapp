import { defineCommand } from '../../registry';
import { archive } from './_issue';

defineCommand({ id: 'issue.archive', label: 'Archive', icon: 'archive', group: 'Issue', shortcut: '#', run: archive });
