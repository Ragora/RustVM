pub struct AbstractSyntaxTree
{
    pub nodes: Vec<ASTNode>
}

pub enum LHSASTNode
{
    LocalVariable {
        name: Vec<String>
    },

    GlobalVariable {
        name: Vec<String>
    }
}

pub enum ControlASTNode
{
    Return {
        expression: Box<ASTNode>
    },

    Break {
        
    },

    Continue {

    },

    While {
        expression: Box<ASTNode>,
        body: Vec<ASTNode>
    },
    
    // %local or $global = ...
    Assign {
        lhs: LHSASTNode,
        rhs: GenericValue
    },

    If {
        expression: Box<ASTNode>,
        body: Vec<ASTNode>,
        else_ifs: Vec<ElseIfASTNode>,
        else_body: Option<Vec<ASTNode>>
    },

    // Form: for (initializer; expression; advance)
    ForLoop {
        initializer: Box<ASTNode>,
        expression: Box<ASTNode>,
        advance: Box<ASTNode>
    }
}

pub struct ElseIfASTNode
{
    pub expression: Box<ASTNode>,
    pub body: Vec<ASTNode>
}

pub enum OpNode
{
    Add {
        lhs: GenericValue,
        rhs: GenericValue
    },

    Subtract {
        lhs: GenericValue,
        rhs: GenericValue
    },

    Multiply {
        lhs: GenericValue,
        rhs: GenericValue
    }
}

/// RHS only nodes - these cannot be LHS
pub enum RHSASTNode
{
    Float {
        value: f32,
    },

    String {
        value: String
    },

    Integer {
        value: i32
    },

    /// Ternary value
    Ternary {
        expression: Box<RHSASTNode>,
        value: Box<RHSASTNode>
    },

    /// Expression RHS
    Expression {
        expression: Box<RHSASTNode>
    }
}

/// Any value; like a constant value or a variable reference
pub enum GenericValue
{
    LHS(LHSASTNode),
    RHS(RHSASTNode)
}

pub enum ASTNode
{
    /// Function declaration block
    FunctionDeclaration {
        name: String,
        namespaces: Vec<String>,
        parameters: Vec<String>,
        body: Vec<ASTNode>
    }
}