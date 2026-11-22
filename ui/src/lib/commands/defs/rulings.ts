import { unbind } from '../registry';

// Ruling: `l` opens the labels page in list, table and issue detail; it moves a column only on the board.
// The table's own next-column command keeps its arrow key; this line goes when its file drops `l`.
unbind('table.right', 'l');
