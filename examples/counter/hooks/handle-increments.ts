interface Block {
    transactions: Array<Transaction>,
}
interface Context {
}
interface Transaction {}
interface ChainUpdatedWithBlock {
    block: Block,
}
interface ChainUpdatedWithReorg {
    oldBlocks: Array<Block>,
    newBlocks: Array<Block>
}

type ChainEvent = ChainUpdatedWithBlock | ChainUpdatedWithReorg;

interface HookSettings {
    name: string,
    chain: Chain,
    filter(transaction: Transaction): Promise<void>;
    run(context: Context, event: ChainEvent): Promise<void>;
}
type HookFunction = (
    event: ChainEvent,
) => void | Promise<void>;

class Lambda {
    static register(settings: HookSettings) {
    };
}

class Keystore {
    static async sign(key: string, transaction: Transaction) {
    };
}

enum Chain {
    Bitcoin,
    Stacks,
}

// HiroBlockhainServices, hbs-kit/hooks, hbs-kit/rosetta/types, hbs/stacks/clarity-types.

// HBS Hooks
// HBS Secret
// HBS Storage



Lambda.register({
    name: "Handle increments",
    chain: Chain.Stacks,
    filter: null,
    run: async (context: Context, event: ChainEvent) => {
        // HTTP POST event, Submit transactions, etc.
    },
})

Lambda.register({
    name: "Handle increments",
    chain: Chain.Stacks,
    filter: null,
    lambda: async (context: Context, event: ChainEvent) => {
        // HTTP POST event, Submit transactions, etc.
    },
});