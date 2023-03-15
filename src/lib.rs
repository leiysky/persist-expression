use std::collections::HashMap;

/// A two dimensional table of integers.
pub struct Table {
    pub name: String,
    pub data: Vec<Vec<i32>>,
}

pub type TableSet = HashMap<String, Table>;

impl Table {
    pub fn new(name: String, data: Vec<Vec<i32>>) -> Self {
        Self { name, data }
    }

    pub fn get(&self, x: usize, y: usize) -> Option<&i32> {
        self.data.get(y).and_then(|row| row.get(x))
    }

    pub fn set(&mut self, x: usize, y: usize, value: i32) {
        if let Some(row) = self.data.get_mut(y) {
            if let Some(cell) = row.get_mut(x) {
                *cell = value;
            }
        }
    }
}

/// Computed Expression
pub enum Expression {
    Number(i32),
    Reference { table: String, x: usize, y: usize },
    Sum(Vec<Expression>),
}

impl Expression {
    pub fn eval(&self, table_set: &TableSet) -> i32 {
        match self {
            Expression::Number(v) => *v,
            Expression::Reference { table, x, y } => table_set
                .get(table)
                .and_then(|table| table.get(*x, *y).copied())
                .unwrap(),
            Expression::Sum(args) => args.iter().fold(0, |acc, v| v.eval(table_set) + acc),
        }
    }
}

pub enum PersistentExpression {
    Number(i32),
    Reference {
        /// Value state
        state: i32,
        /// Identifier
        table: String,
        x: usize,
        y: usize,
    },
    Sum {
        /// Accumulation state
        state: i32,
        args: Vec<PersistentExpression>,
    },
}

impl PersistentExpression {
    pub fn state(&self) -> i32 {
        match self {
            PersistentExpression::Number(v) => *v,
            PersistentExpression::Reference { state, .. } => *state,
            PersistentExpression::Sum { state, .. } => *state,
        }
    }

    /// Apply event, return true if current state is modified.
    pub fn apply(&mut self, event: &TableEvent) -> bool {
        match self {
            PersistentExpression::Number(_) => false,
            PersistentExpression::Reference { state, table, x, y } => {
                let TableEvent::SetValue {
                    table: t,
                    x: x1,
                    y: y1,
                    value,
                } = event;
                if t == table && x == x1 && y == y1 {
                    *state = *value;
                    true
                } else {
                    false
                }
            }
            PersistentExpression::Sum { state, args } => {
                let mut modified = false;
                for arg in args.iter_mut() {
                    let original_state = arg.state();
                    if arg.apply(event) {
                        *state += arg.state() - original_state;
                        modified = true;
                    } else {
                        continue;
                    }
                }

                modified
            }
        }
    }

    /// Initialize state of persistent expression
    pub fn init(&mut self, table_set: &TableSet) {
        match self {
            PersistentExpression::Number(_) => {}
            PersistentExpression::Reference { state, table, x, y } => {
                *state = table_set
                    .get(table)
                    .and_then(|table| table.get(*x, *y).copied())
                    .unwrap();
            }
            PersistentExpression::Sum { state, args } => {
                *state = args.iter_mut().fold(0, |acc, v| {
                    v.init(table_set);
                    v.state() + acc
                });
            }
        }
    }
}

pub enum TableEvent {
    SetValue {
        table: String,
        x: usize,
        y: usize,
        value: i32,
    },
}

#[test]
fn test() {
    let mut table_set = TableSet::new();
    let t1 = Table::new("t1".to_string(), vec![vec![1, 2, 3]]);
    let t2 = Table::new("t2".to_string(), vec![vec![1, 2, 3]]);

    table_set.insert("t1".to_string(), t1);
    table_set.insert("t2".to_string(), t2);

    let expr = Expression::Sum(vec![
        Expression::Number(1),
        Expression::Reference {
            table: "t1".to_string(),
            x: 1,
            y: 0,
        },
    ]);

    let result = expr.eval(&table_set);

    assert_eq!(result, 3);
}

#[test]
fn test_persistent() {
    let mut table_set = TableSet::new();
    let t1 = Table::new("t1".to_string(), vec![vec![1, 2, 3]]);
    let t2 = Table::new("t2".to_string(), vec![vec![1, 2, 3]]);

    table_set.insert("t1".to_string(), t1);
    table_set.insert("t2".to_string(), t2);

    let mut expr = PersistentExpression::Sum {
        state: 0,
        args: vec![
            PersistentExpression::Number(1),
            PersistentExpression::Reference {
                state: 0,
                table: "t1".to_string(),
                x: 1,
                y: 0,
            },
        ],
    };

    expr.init(&table_set);

    assert_eq!(expr.state(), 3);

    table_set.get_mut("t1").unwrap().set(1, 0, 3);
    let event = TableEvent::SetValue {
        table: "t1".to_string(),
        x: 1,
        y: 0,
        value: 3,
    };

    assert!(expr.apply(&event));

    assert_eq!(expr.state(), 4);
}
