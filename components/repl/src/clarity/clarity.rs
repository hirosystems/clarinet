use crate::clarity::analysis;
use crate::clarity::analysis::AnalysisDatabase;
use crate::clarity::analysis::{errors::CheckError, errors::CheckErrors, ContractAnalysis};
use crate::clarity::ast;
use crate::clarity::ast::{errors::ParseError, errors::ParseErrors, ContractAST};
use crate::clarity::contexts::{AssetMap, Environment, OwnedEnvironment};
use crate::clarity::costs::{CostTracker, ExecutionCost, LimitedCostTracker};
use crate::clarity::database::Datastore;
use crate::clarity::database::{
    ClarityBackingStore, ClarityDatabase, HeadersDB, RollbackWrapper, RollbackWrapperPersistedLog,
};
use crate::clarity::errors::Error as InterpreterError;
use crate::clarity::representations::SymbolicExpression;
use crate::clarity::types::{
    AssetIdentifier, PrincipalData, QualifiedContractIdentifier, TypeSignature, Value,
};

use crate::clarity::events::StacksTransactionEvent;
use crate::clarity::StacksBlockId;

use std::error;
use std::fmt;

use super::database;

///
/// A high-level interface for interacting with the Clarity VM.
///
/// ClarityInstance takes ownership of a MARF + Sqlite store used for
///   it's data operations.
/// The ClarityInstance defines a `begin_block(bhh, bhh, bhh) -> ClarityBlockConnection`
///    function.
/// ClarityBlockConnections are used for executing transactions within the context of
///    a single block.
/// Only one ClarityBlockConnection may be open at a time (enforced by the borrow checker)
///   and ClarityBlockConnections must be `commit_block`ed or `rollback_block`ed before discarding
///   begining the next connection (enforced by runtime panics).
///
pub struct ClarityInstance {
    datastore: Option<Datastore>,
    block_limit: ExecutionCost,
    mainnet: bool,
}

///
/// A high-level interface for Clarity VM interactions within a single block.
///
pub struct ClarityBlockConnection<'a> {
    datastore: Datastore,
    parent: &'a mut ClarityInstance,
    header_db: &'a dyn HeadersDB,
    cost_track: Option<LimitedCostTracker>,
    mainnet: bool,
}

///
/// Interface for Clarity VM interactions within a given transaction.
///
///   commit the transaction to the block with .commit()
///   rollback the transaction by dropping this struct.
pub struct ClarityTransactionConnection<'a> {
    log: Option<RollbackWrapperPersistedLog>,
    store: &'a mut Datastore,
    header_db: &'a dyn HeadersDB,
    cost_track: &'a mut Option<LimitedCostTracker>,
    mainnet: bool,
}

pub struct ClarityReadOnlyConnection<'a> {
    datastore: Datastore,
    parent: &'a mut ClarityInstance,
    header_db: &'a dyn HeadersDB,
}

#[derive(Debug)]
pub enum Error {
    Analysis(CheckError),
    Parse(ParseError),
    Interpreter(InterpreterError),
    BadTransaction(String),
    CostError(ExecutionCost, ExecutionCost),
    AbortedByCallback(Option<Value>, AssetMap, Vec<StacksTransactionEvent>),
}

impl From<CheckError> for Error {
    fn from(e: CheckError) -> Self {
        match e.err {
            CheckErrors::CostOverflow => {
                Error::CostError(ExecutionCost::max_value(), ExecutionCost::max_value())
            }
            CheckErrors::CostBalanceExceeded(a, b) => Error::CostError(a, b),
            CheckErrors::MemoryBalanceExceeded(_a, _b) => {
                Error::CostError(ExecutionCost::max_value(), ExecutionCost::max_value())
            }
            _ => Error::Analysis(e),
        }
    }
}

impl From<InterpreterError> for Error {
    fn from(e: InterpreterError) -> Self {
        match &e {
            InterpreterError::Unchecked(CheckErrors::CostBalanceExceeded(a, b)) => {
                Error::CostError(a.clone(), b.clone())
            }
            InterpreterError::Unchecked(CheckErrors::CostOverflow) => {
                Error::CostError(ExecutionCost::max_value(), ExecutionCost::max_value())
            }
            _ => Error::Interpreter(e),
        }
    }
}

impl From<ParseError> for Error {
    fn from(e: ParseError) -> Self {
        match e.err {
            ParseErrors::CostOverflow => {
                Error::CostError(ExecutionCost::max_value(), ExecutionCost::max_value())
            }
            ParseErrors::CostBalanceExceeded(a, b) => Error::CostError(a, b),
            ParseErrors::MemoryBalanceExceeded(_a, _b) => {
                Error::CostError(ExecutionCost::max_value(), ExecutionCost::max_value())
            }
            _ => Error::Parse(e),
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Error::CostError(ref a, ref b) => {
                write!(f, "Cost Error: {} cost exceeded budget of {} cost", a, b)
            }
            Error::Analysis(ref e) => fmt::Display::fmt(e, f),
            Error::Parse(ref e) => fmt::Display::fmt(e, f),
            Error::AbortedByCallback(..) => write!(f, "Post condition aborted transaction"),
            Error::Interpreter(ref e) => fmt::Display::fmt(e, f),
            Error::BadTransaction(ref s) => fmt::Display::fmt(s, f),
        }
    }
}

impl error::Error for Error {
    fn cause(&self) -> Option<&dyn error::Error> {
        match *self {
            Error::CostError(ref _a, ref _b) => None,
            Error::AbortedByCallback(..) => None,
            Error::Analysis(ref e) => Some(e),
            Error::Parse(ref e) => Some(e),
            Error::Interpreter(ref e) => Some(e),
            Error::BadTransaction(ref _s) => None,
        }
    }
}

/// A macro for doing take/replace on a closure.
///   macro is needed rather than a function definition because
///   otherwise, we end up breaking the borrow checker when
///   passing a mutable reference across a function boundary.
macro_rules! using {
    ($to_use: expr, $msg: expr, $exec: expr) => {{
        let object = $to_use.take().expect(&format!(
            "BUG: Transaction connection lost {} handle.",
            $msg
        ));
        let (object, result) = ($exec)(object);
        $to_use.replace(object);
        result
    }};
}

impl ClarityBlockConnection<'_> {
    /// Reset the block's total execution to the given cost, if there is a cost tracker at all.
    /// Used by the miner to "undo" applying a transaction that exceeded the budget.
    pub fn reset_block_cost(&mut self, cost: ExecutionCost) -> () {
        if let Some(ref mut cost_tracker) = self.cost_track {
            cost_tracker.set_total(cost);
        }
    }

    pub fn set_cost_tracker(&mut self, tracker: LimitedCostTracker) -> LimitedCostTracker {
        let old = self
            .cost_track
            .take()
            .expect("BUG: Clarity block connection lost cost tracker instance");
        self.cost_track.replace(tracker);
        old
    }

    /// Get the current cost so far
    pub fn cost_so_far(&self) -> ExecutionCost {
        match self.cost_track {
            Some(ref track) => track.get_total(),
            None => ExecutionCost::zero(),
        }
    }
}

impl ClarityInstance {
    pub fn new(mainnet: bool, datastore: Datastore, block_limit: ExecutionCost) -> ClarityInstance {
        ClarityInstance {
            datastore: Some(datastore),
            block_limit,
            mainnet,
        }
    }

    pub fn begin_block<'a>(
        &'a mut self,
        current: &StacksBlockId,
        next: &StacksBlockId,
        header_db: &'a dyn HeadersDB,
    ) -> ClarityBlockConnection<'a> {
        let mut datastore = self
            .datastore
            .take()
            // this is a panicking failure, because there should be _no instance_ in which a ClarityBlockConnection
            //   doesn't restore it's parent's datastore
            .expect(
                "FAIL: use of begin_block while prior block neither committed nor rolled back.",
            );

        datastore.begin(current, next);

        let cost_track = Some(LimitedCostTracker::new_free());

        ClarityBlockConnection {
            datastore,
            header_db,
            parent: self,
            cost_track,
            mainnet: false,
        }
    }

    pub fn read_only_connection<'a>(
        &'a mut self,
        at_block: &StacksBlockId,
        header_db: &'a dyn HeadersDB,
    ) -> ClarityReadOnlyConnection<'a> {
        let mut datastore = self
            .datastore
            .take()
            // this is a panicking failure, because there should be _no instance_ in which a ClarityBlockConnection
            //   doesn't restore it's parent's datastore
            .expect(
                "FAIL: use of begin_block while prior block neither committed nor rolled back.",
            );

        datastore.set_chain_tip(at_block);

        ClarityReadOnlyConnection {
            datastore,
            header_db,
            parent: self,
        }
    }

    pub fn eval_read_only(
        &mut self,
        at_block: &StacksBlockId,
        header_db: &dyn HeadersDB,
        contract: &QualifiedContractIdentifier,
        program: &str,
    ) -> Result<Value, Error> {
        self.datastore.as_mut().unwrap().set_chain_tip(at_block);
        let clarity_db = self.datastore.as_mut().unwrap().as_clarity_db(header_db);
        let mut env = OwnedEnvironment::new_free(false, clarity_db);
        env.eval_read_only(contract, program)
            .map(|(x, _, _)| x)
            .map_err(Error::from)
    }

    pub fn destroy(mut self) -> Datastore {
        let datastore = self.datastore.take()
            .expect("FAIL: attempt to recover database connection from clarity instance which is still open");

        datastore
    }
}

pub trait ClarityConnection {
    /// Do something to the underlying DB that involves only reading.
    fn with_clarity_db_readonly_owned<F, R>(&mut self, to_do: F) -> R
    where
        F: FnOnce(ClarityDatabase) -> (R, ClarityDatabase);
    fn with_analysis_db_readonly<F, R>(&mut self, to_do: F) -> R
    where
        F: FnOnce(&mut AnalysisDatabase) -> R;

    fn with_clarity_db_readonly<F, R>(&mut self, to_do: F) -> R
    where
        F: FnOnce(&mut ClarityDatabase) -> R,
    {
        self.with_clarity_db_readonly_owned(|mut db| (to_do(&mut db), db))
    }

    fn with_readonly_clarity_env<F, R>(
        &mut self,
        mainnet: bool,
        sender: PrincipalData,
        cost_track: LimitedCostTracker,
        to_do: F,
    ) -> Result<R, InterpreterError>
    where
        F: FnOnce(&mut Environment) -> Result<R, InterpreterError>,
    {
        self.with_clarity_db_readonly_owned(|clarity_db| {
            let mut vm_env = OwnedEnvironment::new_cost_limited(mainnet, clarity_db, cost_track);
            let result = vm_env
                .execute_in_env(sender.into(), to_do)
                .map(|(result, _, _)| result);
            let (db, _) = vm_env
                .destruct()
                .expect("Failed to recover database reference after executing transaction");
            (result, db)
        })
    }
}

impl ClarityConnection for ClarityBlockConnection<'_> {
    /// Do something with ownership of the underlying DB that involves only reading.
    fn with_clarity_db_readonly_owned<F, R>(&mut self, to_do: F) -> R
    where
        F: FnOnce(ClarityDatabase) -> (R, ClarityDatabase),
    {
        let mut db = ClarityDatabase::new(&mut self.datastore, self.header_db);
        db.begin();
        let (result, mut db) = to_do(db);
        db.roll_back();
        result
    }

    fn with_analysis_db_readonly<F, R>(&mut self, to_do: F) -> R
    where
        F: FnOnce(&mut AnalysisDatabase) -> R,
    {
        let mut db = AnalysisDatabase::new(&mut self.datastore);
        db.begin();
        let result = to_do(&mut db);
        db.roll_back();
        result
    }
}

impl ClarityConnection for ClarityReadOnlyConnection<'_> {
    /// Do something with ownership of the underlying DB that involves only reading.
    fn with_clarity_db_readonly_owned<F, R>(&mut self, to_do: F) -> R
    where
        F: FnOnce(ClarityDatabase) -> (R, ClarityDatabase),
    {
        let mut db = ClarityDatabase::new(&mut self.datastore, self.header_db);
        db.begin();
        let (result, mut db) = to_do(db);
        db.roll_back();
        result
    }

    fn with_analysis_db_readonly<F, R>(&mut self, to_do: F) -> R
    where
        F: FnOnce(&mut AnalysisDatabase) -> R,
    {
        let mut db = AnalysisDatabase::new(&mut self.datastore);
        db.begin();
        let result = to_do(&mut db);
        db.roll_back();
        result
    }
}

impl<'a> ClarityReadOnlyConnection<'a> {
    pub fn done(self) {
        self.parent.datastore.replace(self.datastore);
    }
}

impl<'a> ClarityBlockConnection<'a> {
    /// Rolls back all changes in the current block by
    /// (1) dropping all writes from the current MARF tip,
    /// (2) rolling back side-storage
    pub fn rollback_block(mut self) {
        // this is a "lower-level" rollback than the roll backs performed in
        //   ClarityDatabase or AnalysisDatabase -- this is done at the backing store level.
        println!("Rollback Clarity datastore");
        self.datastore.rollback();

        self.parent.datastore.replace(self.datastore);
    }

    /// Commits all changes in the current block by
    /// (1) committing the current MARF tip to storage,
    /// (2) committing side-storage.

    /// Commits all changes in the current block by
    /// (1) committing the current MARF tip to storage,
    /// (2) committing side-storage.  Commits to a different
    /// block hash than the one opened (i.e. since the caller
    /// may not have known the "real" block hash at the
    /// time of opening).
    pub fn commit_to_block(mut self, final_bhh: &StacksBlockId) -> LimitedCostTracker {
        println!("Commit Clarity datastore to {:?}", final_bhh);
        self.datastore.commit_to(final_bhh);

        self.parent.datastore.replace(self.datastore);

        self.cost_track.unwrap()
    }

    /// Commits all changes in the current block by
    /// (1) committing the current MARF tip to storage,
    /// (2) committing side-storage.
    ///    before this saves, it updates the metadata headers in
    ///    the sidestore so that they don't get stepped on after
    ///    a miner re-executes a constructed block.
    pub fn commit_mined_block(mut self, bhh: &StacksBlockId) -> LimitedCostTracker {
        println!("Commit mined Clarity datastore to {:?}", bhh);
        self.datastore.commit_mined_block(bhh);

        self.parent.datastore.replace(self.datastore);

        self.cost_track.unwrap()
    }

    pub fn start_transaction_processing<'b>(&'b mut self) -> ClarityTransactionConnection<'b> {
        let store = &mut self.datastore;
        let cost_track = &mut self.cost_track;
        let header_db = self.header_db;
        let mainnet = self.mainnet;
        let mut log = RollbackWrapperPersistedLog::new();
        log.nest();
        ClarityTransactionConnection {
            store,
            cost_track,
            header_db,
            log: Some(log),
            mainnet,
        }
    }

    pub fn as_transaction<F, R>(&mut self, todo: F) -> R
    where
        F: FnOnce(&mut ClarityTransactionConnection) -> R,
    {
        let mut tx = self.start_transaction_processing();
        let r = todo(&mut tx);
        tx.commit();
        r
    }
}

impl ClarityConnection for ClarityTransactionConnection<'_> {
    /// Do something with ownership of the underlying DB that involves only reading.
    fn with_clarity_db_readonly_owned<F, R>(&mut self, to_do: F) -> R
    where
        F: FnOnce(ClarityDatabase) -> (R, ClarityDatabase),
    {
        using!(self.log, "log", |log| {
            let rollback_wrapper = RollbackWrapper::from_persisted_log(self.store, log);
            let mut db =
                ClarityDatabase::new_with_rollback_wrapper(rollback_wrapper, self.header_db);
            db.begin();
            let (r, mut db) = to_do(db);
            db.roll_back();
            (db.destroy().into(), r)
        })
    }

    fn with_analysis_db_readonly<F, R>(&mut self, to_do: F) -> R
    where
        F: FnOnce(&mut AnalysisDatabase) -> R,
    {
        self.inner_with_analysis_db(|mut db| {
            db.begin();
            let result = to_do(&mut db);
            db.roll_back();
            result
        })
    }
}

impl<'a> Drop for ClarityTransactionConnection<'a> {
    fn drop(&mut self) {
        self.cost_track
            .as_mut()
            .expect("BUG: Transaction connection lost cost_tracker handle.")
            .reset_memory();
    }
}

impl<'a> ClarityTransactionConnection<'a> {
    fn inner_with_analysis_db<F, R>(&mut self, to_do: F) -> R
    where
        F: FnOnce(&mut AnalysisDatabase) -> R,
    {
        using!(self.log, "log", |log| {
            let rollback_wrapper = RollbackWrapper::from_persisted_log(self.store, log);
            let mut db = AnalysisDatabase::new_with_rollback_wrapper(rollback_wrapper);
            let r = to_do(&mut db);
            (db.destroy().into(), r)
        })
    }

    /// Do something to the underlying DB that involves writing.
    pub fn with_clarity_db<F, R>(&mut self, to_do: F) -> Result<R, Error>
    where
        F: FnOnce(&mut ClarityDatabase) -> Result<R, Error>,
    {
        using!(self.log, "log", |log| {
            let rollback_wrapper = RollbackWrapper::from_persisted_log(self.store, log);
            let mut db =
                ClarityDatabase::new_with_rollback_wrapper(rollback_wrapper, self.header_db);

            db.begin();
            let result = to_do(&mut db);
            if result.is_ok() {
                db.commit();
            } else {
                db.roll_back();
            }

            (db.destroy().into(), result)
        })
    }

    /// What's our total (block-wide) resource use so far?
    pub fn cost_so_far(&self) -> ExecutionCost {
        match self.cost_track {
            Some(ref track) => track.get_total(),
            None => ExecutionCost::zero(),
        }
    }

    /// Analyze a provided smart contract, but do not write the analysis to the AnalysisDatabase
    pub fn analyze_smart_contract(
        &mut self,
        identifier: &QualifiedContractIdentifier,
        contract_content: &str,
    ) -> Result<(ContractAST, ContractAnalysis), Error> {
        using!(self.cost_track, "cost tracker", |mut cost_track| {
            self.inner_with_analysis_db(|db| {
                let ast_result = ast::build_ast(identifier, contract_content, &mut cost_track);

                let mut contract_ast = match ast_result {
                    Ok(x) => x,
                    Err(e) => return (cost_track, Err(e.into())),
                };

                let result = analysis::run_analysis(
                    identifier,
                    &mut contract_ast.expressions,
                    db,
                    false,
                    cost_track,
                );

                match result {
                    Ok(mut contract_analysis) => {
                        let cost_track = contract_analysis.take_contract_cost_tracker();
                        (cost_track, Ok((contract_ast, contract_analysis)))
                    }
                    Err((e, cost_track)) => (cost_track, Err(e.into())),
                }
            })
        })
    }

    fn with_abort_callback<F, A, R>(
        &mut self,
        to_do: F,
        abort_call_back: A,
    ) -> Result<(R, AssetMap, Vec<StacksTransactionEvent>, bool), Error>
    where
        A: FnOnce(&AssetMap, &mut ClarityDatabase) -> bool,
        F: FnOnce(
            &mut OwnedEnvironment,
        ) -> Result<(R, AssetMap, Vec<StacksTransactionEvent>), Error>,
    {
        using!(self.log, "log", |log| {
            using!(self.cost_track, "cost tracker", |cost_track| {
                let rollback_wrapper = RollbackWrapper::from_persisted_log(self.store, log);
                let mut db =
                    ClarityDatabase::new_with_rollback_wrapper(rollback_wrapper, self.header_db);

                // wrap the whole contract-call in a claritydb transaction,
                //   so we can abort on call_back's boolean retun
                db.begin();
                let mut vm_env = OwnedEnvironment::new_cost_limited(false, db, cost_track);
                let result = to_do(&mut vm_env);
                let (mut db, cost_track) = vm_env
                    .destruct()
                    .expect("Failed to recover database reference after executing transaction");
                // DO NOT reset memory usage yet -- that should happen only when the TX commits.

                let result = match result {
                    Ok((value, asset_map, events)) => {
                        let aborted = abort_call_back(&asset_map, &mut db);
                        if aborted {
                            db.roll_back();
                        } else {
                            db.commit();
                        }
                        Ok((value, asset_map, events, aborted))
                    }
                    Err(e) => {
                        db.roll_back();
                        Err(e)
                    }
                };

                (cost_track, (db.destroy().into(), result))
            })
        })
    }

    /// Save a contract analysis output to the AnalysisDatabase
    /// An error here would indicate that something has gone terribly wrong in the processing of a contract insert.
    ///   the caller should likely abort the whole block or panic
    pub fn save_analysis(
        &mut self,
        identifier: &QualifiedContractIdentifier,
        contract_analysis: &ContractAnalysis,
    ) -> Result<(), CheckError> {
        self.inner_with_analysis_db(|db| {
            db.begin();
            let result = db.insert_contract(identifier, contract_analysis);
            match result {
                Ok(_) => {
                    db.commit();
                    Ok(())
                }
                Err(e) => {
                    db.roll_back();
                    Err(e)
                }
            }
        })
    }

    /// Execute a STX transfer in the current block.
    /// Will throw an error if it tries to spend STX that the 'from' principal doesn't have.
    pub fn run_stx_transfer(
        &mut self,
        from: &PrincipalData,
        to: &PrincipalData,
        amount: u128,
    ) -> Result<(Value, AssetMap, Vec<StacksTransactionEvent>), Error> {
        self.with_abort_callback(
            |vm_env| vm_env.stx_transfer(from, to, amount).map_err(Error::from),
            |_, _| false,
        )
        .and_then(|(value, assets, events, _)| Ok((value, assets, events)))
    }

    /// Execute a contract call in the current block.
    ///  If an error occurs while processing the transaction, it's modifications will be rolled back.
    /// abort_call_back is called with an AssetMap and a ClarityDatabase reference,
    ///   if abort_call_back returns true, all modifications from this transaction will be rolled back.
    ///      otherwise, they will be committed (though they may later be rolled back if the block itself is rolled back).
    pub fn run_contract_call<F>(
        &mut self,
        sender: &PrincipalData,
        contract: &QualifiedContractIdentifier,
        public_function: &str,
        args: &[Value],
        abort_call_back: F,
    ) -> Result<(Value, AssetMap, Vec<StacksTransactionEvent>), Error>
    where
        F: FnOnce(&AssetMap, &mut ClarityDatabase) -> bool,
    {
        let expr_args: Vec<_> = args
            .iter()
            .map(|x| SymbolicExpression::atom_value(x.clone()))
            .collect();

        self.with_abort_callback(
            |vm_env| {
                vm_env
                    .execute_transaction(
                        sender.clone(),
                        contract.clone(),
                        public_function,
                        &expr_args,
                    )
                    .map_err(Error::from)
            },
            abort_call_back,
        )
        .and_then(|(value, assets, events, aborted)| {
            if aborted {
                Err(Error::AbortedByCallback(Some(value), assets, events))
            } else {
                Ok((value, assets, events))
            }
        })
    }

    /// Initialize a contract in the current block.
    ///  If an error occurs while processing the initialization, it's modifications will be rolled back.
    /// abort_call_back is called with an AssetMap and a ClarityDatabase reference,
    ///   if abort_call_back returns true, all modifications from this transaction will be rolled back.
    ///      otherwise, they will be committed (though they may later be rolled back if the block itself is rolled back).
    pub fn initialize_smart_contract<F>(
        &mut self,
        identifier: &QualifiedContractIdentifier,
        contract_ast: &ContractAST,
        contract_str: &str,
        abort_call_back: F,
    ) -> Result<(AssetMap, Vec<StacksTransactionEvent>), Error>
    where
        F: FnOnce(&AssetMap, &mut ClarityDatabase) -> bool,
    {
        let (_, asset_map, events, aborted) = self.with_abort_callback(
            |vm_env| {
                vm_env
                    .initialize_contract_from_ast(identifier.clone(), contract_ast, contract_str)
                    .map_err(Error::from)
            },
            abort_call_back,
        )?;
        if aborted {
            Err(Error::AbortedByCallback(None, asset_map, events))
        } else {
            Ok((asset_map, events))
        }
    }

    /// Commit the changes from the edit log.
    /// panics if there is more than one open savepoint
    pub fn commit(mut self) {
        let log = self
            .log
            .take()
            .expect("BUG: Transaction Connection lost db log connection.");
        let mut rollback_wrapper = RollbackWrapper::from_persisted_log(self.store, log);
        if rollback_wrapper.depth() != 1 {
            panic!(
                "Attempted to commit transaction with {} != 1 rollbacks",
                rollback_wrapper.depth()
            );
        }
        rollback_wrapper.commit();
        // now we can reset the memory usage for the edit-log
        self.cost_track
            .as_mut()
            .expect("BUG: Transaction connection lost cost tracker connection.")
            .reset_memory();
    }

    /// Evaluate a raw Clarity snippit
    #[cfg(test)]
    pub fn clarity_eval_raw(&mut self, code: &str) -> Result<Value, Error> {
        let (result, _, _, _) = self.with_abort_callback(
            |vm_env| vm_env.eval_raw(code).map_err(Error::from),
            |_, _| false,
        )?;
        Ok(result)
    }

    #[cfg(test)]
    pub fn eval_read_only(
        &mut self,
        contract: &QualifiedContractIdentifier,
        code: &str,
    ) -> Result<Value, Error> {
        let (result, _, _, _) = self.with_abort_callback(
            |vm_env| vm_env.eval_read_only(contract, code).map_err(Error::from),
            |_, _| false,
        )?;
        Ok(result)
    }
}
