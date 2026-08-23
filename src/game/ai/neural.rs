use std::array::from_fn;
use std::fmt::{Debug, Display, Formatter};
use std::ops::{Add, AddAssign};
use rand::distr::{Distribution, StandardUniform};
use rand::{Rng, RngExt};
use crate::game::ai::coefficient::Coefficient;
use crate::game::ai::genome::Genome;

#[derive(Copy, Clone, PartialEq)]
pub struct Tensor<const R: usize, const C: usize = 1> {
    data: [[f64; C]; R],
}

impl<const R: usize, const C: usize> Tensor<R, C> {

    pub fn rows(&self) -> usize {
        R
    }

    pub fn cols(&self) -> usize {
        C
    }


    const ZEROS: Self = Self {
        data: [[0.0; C]; R],
    };

    const ONES: Self = Self {
        data: [[1.0; C]; R],
    };

    pub fn new(data: [[f64; C]; R]) -> Self {
        Self { data }
    }

    pub const TOTAL_SIZE: usize = R * C;

    pub fn from_slice(data: &[f64]) -> Self {
        debug_assert_eq!(data.len(), Self::TOTAL_SIZE,
           "Invalid data length for Tensor<{}, {}>: expected {}, got {}",
           R, C, Self::TOTAL_SIZE, data.len()
        );
        let mut result = Self::ZEROS;
        for i in 0..R {
            for j in 0..C {
                result.data[i][j] = data[i * C + j];
            }
        }
        result
    }

    pub fn flatten(&self) -> Vec<f64> {
        let mut result = Vec::with_capacity(Self::TOTAL_SIZE);
        for row in self.data.iter() {
            for col in row.iter() {
                result.push(*col);
            }
        }
        result
    }
    
    pub fn dot<const R2: usize, const C2: usize>(&self, other: &Tensor<R2, C2>) -> Tensor<R, C2> {
        debug_assert_eq!(C, R2, "Cannot multiply tensors with incompatible dimensions");

        let mut result = Tensor::ZEROS;

        for i in 0..self.rows() {
            for j in 0..other.cols() {
                for k in 0..self.cols() {
                    result.data[i][j] += self.data[i][k] * other.data[k][j];
                }
            }
        }

        result
    }
    fn relu_mut(&mut self) {
        for i in 0..self.rows() {
            for j in 0..self.cols() {
                self.data[i][j] = relu(self.data[i][j]);
            }
        }
    }

    fn sigmoid_mut(&mut self) {
        for i in 0..self.rows() {
            for j in 0..self.cols() {
                self.data[i][j] = sigmoid(self.data[i][j]);
            }
        }
    }

    fn mcculloch_pitts_mut(&mut self, threshold: f64) {
        for i in 0..self.rows() {
            for j in 0..self.cols() {
                self.data[i][j] = mcculloch_pitts(self.data[i][j], threshold);
            }
        }
    }

    fn fmt(&self, f: &mut Formatter<'_>, indent: usize) -> std::fmt::Result {
        let mut formatted_nums = Vec::with_capacity(R * C);
        let mut col_widths = vec![0; C];
        for row in self.data.iter() {
            for (col_idx, val) in row.iter().enumerate() {
                let formatted = format!("{:.6}", val);
                col_widths[col_idx] = col_widths[col_idx].max(formatted.len());
                formatted_nums.push(formatted);
            }
        }

        let indent_str = " ".repeat(indent);
        for i in 0..R {
            if i > 0 {
                writeln!(f)?;
            }
            write!(f, "{}[", indent_str)?;
            for j in 0..C {
                if j > 0 {
                    write!(f, " ")?;
                }
                let num = &formatted_nums[i * C + j];
                write!(f, "{:>width$}", num, width = col_widths[j])?;
            }
            write!(f, "]")?;
        }

        Ok(())
    }
}


fn relu(x: f64) -> f64 {
    x.max(0.0)
}

fn sigmoid(x: f64) -> f64 {
    1.0 / (1.0 + (-x).exp())
}

fn mcculloch_pitts(x: f64, threshold: f64) -> f64 {
    if x > threshold { 1.0 } else { 0.0 }
}

fn activate(x: f64, activation: ActivationFunction) -> f64 {
    match activation {
        ActivationFunction::Identity => x,
        ActivationFunction::ReLU => relu(x),
        ActivationFunction::Sigmoid => sigmoid(x),
        ActivationFunction::McCullochPitt(threshold) => mcculloch_pitts(x, threshold),
    }
}

impl<const SIZE: usize> Tensor<SIZE> {
    pub fn vector(data: [f64; SIZE]) -> Self {
        let mut result = Self::ZEROS;
        for i in 0..SIZE {
            result.data[i][0] = data[i]
        }
        result
    }

    pub fn into_diagonal(self) -> Tensor<SIZE, SIZE> {
        let mut result = Tensor::ZEROS;
        for i in 0..SIZE {
            result.data[i][i] = self.data[i][0]
        }
        result
    }

    fn activate_mut(&mut self, activation: [ActivationFunction; SIZE]) {
        for i in 0..SIZE {
            self.data[i][0] = activate(self.data[i][0], activation[i])
        }
    }
}

impl<const SIZE: usize> Tensor<SIZE, SIZE> {
    pub fn diagonal(data: [f64; SIZE]) -> Self {
        let mut result = Self::ZEROS;
        for i in 0..SIZE {
            result.data[i][i] = data[i]
        }
        result
    }
}

impl Tensor<1, 1> {
    pub fn value(&self) -> f64 {
        self.data[0][0]
    }
}

impl<const R: usize, const C: usize> Add for Tensor<R, C> {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        let mut result = Self::ZEROS;
        for i in 0..R {
            for j in 0..C {
                result.data[i][j] = self.data[i][j] + rhs.data[i][j];
            }
        }
        result
    }
}

impl<const R: usize, const C: usize> AddAssign for Tensor<R, C> {
    fn add_assign(&mut self, rhs: Self) {
        for i in 0..R {
            for j in 0..C {
                self.data[i][j] += rhs.data[i][j];
            }
        }
    }
}


impl<const R: usize, const C: usize> Display for Tensor<R, C> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        self.fmt(f, 0)
    }
}

impl<const R: usize, const C: usize> Debug for Tensor<R, C> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        self.fmt(f, 0)
    }
}


impl<const R: usize, const C: usize> Default for Tensor<R, C> {
    fn default() -> Self {
        Self::ZEROS
    }
}

impl<const R: usize, const C: usize> Distribution<Tensor<R, C>> for StandardUniform {
    fn sample<RNG: Rng + ?Sized>(&self, rng: &mut RNG) -> Tensor<R, C> {
        let mut result = Tensor::ZEROS;
        for i in 0..R {
            for j in 0..C {
                result.data[i][j] = rng.random_range(0.0 ..= 1.0);
            }
        }
        result
    }
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum ActivationFunction {
    Identity,
    Sigmoid,
    ReLU,
    McCullochPitt(f64)
}

impl Default for ActivationFunction {
    fn default() -> Self {
        ActivationFunction::Sigmoid
    }
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub struct Layer<const IN: usize, const SIZE: usize> {
    weights: Tensor<SIZE, IN>,
    bias: Tensor<SIZE>,
    activation: [ActivationFunction; SIZE],
}

impl<const IN: usize, const SIZE: usize> Display for Layer<IN, SIZE> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Layer<{}, {}>:", IN, SIZE)?;
        writeln!(f, "  Weights:")?;
        self.weights.fmt(f, 4)?;
        writeln!(f, "\n  Bias:")?;
        self.bias.fmt(f, 4)?;
        write!(f, "\n  Activations: {:?}", self.activation)
    }
}


impl<const IN: usize, const SIZE: usize> Layer<IN, SIZE> {
    pub fn new(weights: Tensor<SIZE, IN>, bias: Tensor<SIZE>, activation: [ActivationFunction; SIZE]) -> Self {
        Self { weights, bias, activation }
    }

    pub fn fully_connected(weights: Tensor<SIZE, IN>, bias: Tensor<SIZE>, activation: ActivationFunction) -> Self {
        Self::new(weights, bias, [activation; SIZE])
    }

    pub fn mcculloch_pitt(weights: Tensor<SIZE, IN>, thresholds: [f64; SIZE]) -> Self {
        Self::new(weights, Tensor::ZEROS, thresholds.map(ActivationFunction::McCullochPitt))
    }

    pub fn set_activation(&mut self, activation: ActivationFunction) {
        self.activation = [activation; SIZE]
    }

    const WEIGHTS_SIZE: usize = IN * SIZE;
    pub const TOTAL_SIZE: usize = Self::WEIGHTS_SIZE + SIZE; // weights + biases

    pub fn flatten(&self) -> Vec<f64> {
        let mut result = Vec::with_capacity(Self::TOTAL_SIZE);
        result.extend(self.weights.flatten());
        result.extend(self.bias.flatten());
        debug_assert_eq!(result.len(), Self::TOTAL_SIZE, "Layer flattened size mismatch");
        result
    }

    pub fn from_slice(data: &[f64]) -> Self {
        debug_assert_eq!(data.len(), Self::TOTAL_SIZE,
           "Invalid data length for Layer<{}, {}>: expected {}, got {}",
           IN, SIZE, Self::TOTAL_SIZE, data.len()
        );
        Self {
            // First WEIGHTS_SIZE elements are weights
            weights: Tensor::from_slice(&data[..Self::WEIGHTS_SIZE]),
            // Remaining SIZE elements are biases
            bias: Tensor::from_slice(&data[Self::WEIGHTS_SIZE..]),
            // Use default activation function
            activation: [Default::default(); SIZE]
        }
    }

    fn forward(&self, input: &Tensor<IN>) -> Tensor<SIZE> {
        // Perform forward propagation: output = (weights · input) + bias
        let mut result = self.weights.dot(input);
        result += self.bias;
        result.activate_mut(self.activation);
        result
    }

    pub fn backward(&self,
                    input: &Tensor<IN>,
                    output: &Tensor<SIZE>,
                    upstream_gradient: &Tensor<SIZE>
    ) -> (Tensor<SIZE, IN>, Tensor<SIZE>, Tensor<IN>) {
        // First apply activation function derivative
        let mut activation_gradient = *upstream_gradient;
        for i in 0..SIZE {
            activation_gradient.data[i][0] *= match self.activation[i] {
                ActivationFunction::Identity => 1.0,
                ActivationFunction::ReLU => if output.data[i][0] > 0.0 { 1.0 } else { 0.0 },
                ActivationFunction::Sigmoid => {
                    let s = output.data[i][0];
                    s * (1.0 - s) // derivative of sigmoid
                },
                ActivationFunction::McCullochPitt(_) => 0.0, // Not differentiable, treated as 0
            };
        }


        // Calculate gradients
        // dL/dW = dL/dY * X^T
        let mut weight_gradient = Tensor::ZEROS;
        for i in 0..SIZE {
            for j in 0..IN {
                weight_gradient.data[i][j] = activation_gradient.data[i][0] * input.data[j][0];
            }
        }

        // dL/db = dL/dY
        let bias_gradient = activation_gradient;

        // dL/dX = W^T * dL/dY
        let mut input_gradient = Tensor::ZEROS;
        for i in 0..IN {
            for j in 0..SIZE {
                input_gradient.data[i][0] += self.weights.data[j][i] * activation_gradient.data[j][0];
            }
        }

        // TODO type this
        (weight_gradient, bias_gradient, input_gradient)
    }

    pub fn update(&mut self, weight_gradient: &Tensor<SIZE, IN>, bias_gradient: &Tensor<SIZE>, learning_rate: f64) {
        // Update weights: W = W - learning_rate * dL/dW
        for i in 0..SIZE {
            for j in 0..IN {
                self.weights.data[i][j] -= learning_rate * weight_gradient.data[i][j];
            }
        }

        // Update biases: b = b - learning_rate * dL/db
        for i in 0..SIZE {
            self.bias.data[i][0] -= learning_rate * bias_gradient.data[i][0];
        }
    }


}

impl<const IN: usize, const SIZE: usize> Distribution<Layer<IN, SIZE>> for StandardUniform {
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> Layer<IN, SIZE> {
        let scale = (2.0 / (IN + SIZE) as f64).sqrt();
        let mut weights = Tensor::ZEROS;
        let mut bias = Tensor::ZEROS;

        // Xavier/Glorot initialization
        for i in 0..SIZE {
            for j in 0..IN {
                weights.data[i][j] = (rng.random::<f64>() * 2.0 - 1.0) * scale;
            }
            bias.data[i][0] = (rng.random::<f64>() * 2.0 - 1.0) * 0.1;
        }
        Layer { weights, bias, activation: [Default::default(); SIZE] }
    }
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub struct NeuralNetwork<const IN: usize, const HIDDEN: usize, const OUT: usize, const WIDTH: usize> {
    input: Layer<IN, WIDTH>,
    hidden: [Layer<WIDTH, WIDTH>; HIDDEN],
    output: Layer<WIDTH, OUT>,
}

impl<const IN: usize, const HIDDEN: usize, const OUT: usize, const WIDTH: usize> Display for NeuralNetwork<IN, HIDDEN, OUT, WIDTH> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "NeuralNetwork<{}, {}, {}, {}>", IN, OUT, WIDTH, HIDDEN)?;
        writeln!(f, "Input {}", self.input)?;

        for (i, layer) in self.hidden.iter().enumerate() {
            writeln!(f, "Hidden[{}] {}", i + 1, layer)?;
        }

        write!(f, "Output {}", self.output)
    }
}

impl<const IN: usize, const HIDDEN: usize, const OUT: usize, const WIDTH: usize> NeuralNetwork<IN, HIDDEN, OUT, WIDTH> {
    const INPUT_LAYER_SIZE: usize = Layer::<IN, WIDTH>::TOTAL_SIZE;
    const HIDDEN_LAYER_SIZE: usize = Layer::<WIDTH, WIDTH>::TOTAL_SIZE;
    const OUTPUT_LAYER_SIZE: usize = Layer::<WIDTH, OUT>::TOTAL_SIZE;
    pub const TOTAL_SIZE: usize = Self::INPUT_LAYER_SIZE + HIDDEN * Self::HIDDEN_LAYER_SIZE + Self::OUTPUT_LAYER_SIZE;

    pub fn flatten(&self) -> Vec<f64> {
        let mut result = Vec::with_capacity(Self::TOTAL_SIZE);

        // Flatten input layer
        result.extend(self.input.flatten());

        // Flatten hidden layers
        for layer in self.hidden.iter() {
            result.extend(layer.flatten());
        }

        // Flatten output layer
        result.extend(self.output.flatten());

        debug_assert_eq!(result.len(), Self::TOTAL_SIZE, "Network flattened size mismatch");
        result
    }

    pub fn from_slice(data: &[f64]) -> Self {
        debug_assert_eq!(data.len(), Self::TOTAL_SIZE,
             "Invalid data length for NeuralNetwork<{}, {}, {}, {}>: expected {}, got {}",
             IN, HIDDEN, OUT, WIDTH, Self::TOTAL_SIZE, data.len()
        );

        let mut offset = 0;

        // Create input layer
        let input = Layer::from_slice(&data[offset..offset + Self::INPUT_LAYER_SIZE]);
        offset += Self::INPUT_LAYER_SIZE;

        // Create hidden layers
        let mut hidden = Vec::with_capacity(HIDDEN);
        for _ in 0..HIDDEN {
            hidden.push(Layer::from_slice(&data[offset..offset + Self::HIDDEN_LAYER_SIZE]));
            offset += Self::HIDDEN_LAYER_SIZE;
        }
        let hidden = hidden.try_into().unwrap();

        // Create output layer
        let output = Layer::from_slice(&data[offset..offset + Self::OUTPUT_LAYER_SIZE]);

        Self { input, hidden, output }
    }


    pub fn set_input_activation(&mut self, activation: ActivationFunction) {
        self.input.set_activation(activation)
    }

    pub fn set_hidden_activation(&mut self, activation: ActivationFunction) {
        for layer in self.hidden.iter_mut() {
            layer.set_activation(activation);
        }
    }

    pub fn set_output_activation(&mut self, activation: ActivationFunction) {
        self.output.set_activation(activation)
    }

    pub fn set_activation(&mut self, activation: ActivationFunction) {
        self.set_input_activation(activation);
        self.set_hidden_activation(activation);
    }

    pub fn set_default_activation(&mut self) {
        self.set_activation(ActivationFunction::Sigmoid);
        self.set_output_activation(ActivationFunction::Identity);
    }

    pub fn forward(&self, input: &Tensor<IN>) -> Tensor<OUT> {
        let mut current = self.input.forward(input);
        for layer in self.hidden.iter() {
            current = layer.forward(&current);
        }
        self.output.forward(&current)
    }

    pub fn train_step(&mut self, input: &Tensor<IN>, target: &Tensor<OUT>, learning_rate: f64) -> f64 {
        // Store activations during forward pass
        let mut hidden_activations = Vec::with_capacity(HIDDEN);
        let mut hidden_outputs = Vec::with_capacity(HIDDEN);

        // Forward pass

        // input layer
        let initial_activation = *input;
        let mut current = self.input.forward(input);
        let initial_output = current;

        // hidden layers
        for layer in self.hidden.iter() {
            hidden_activations.push(current);
            current = layer.forward(&current);
            hidden_outputs.push(current);
        }

        // output layer
        let final_activation = current;
        let final_output = self.output.forward(&current);

        // Calculate loss and initial gradient
        let mut loss = 0.0;
        let mut output_gradient = Tensor::ZEROS;
        for i in 0..OUT {
            let diff = final_output.data[i][0] - target.data[i][0];
            loss += 0.5 * diff * diff; // MSE loss
            output_gradient.data[i][0] = diff; // derivative of MSE
        }

        // Backward pass
        let (w_grad, b_grad, mut upstream_grad) = self.output.backward(
            &final_activation,
            &final_output,
            &output_gradient
        );
        self.output.update(&w_grad, &b_grad, learning_rate);

        // Backpropagate through hidden layers
        for i in (0..HIDDEN).rev() {
            let (w_grad, b_grad, grad) = self.hidden[i].backward(
                &hidden_activations[i],
                &hidden_outputs[i],
                &upstream_grad
            );
            self.hidden[i].update(&w_grad, &b_grad, learning_rate);
            upstream_grad = grad;
        }

        // Input layer
        let (w_grad, b_grad, _) = self.input.backward(
            &initial_activation,
            &initial_output,
            &upstream_grad
        );
        self.input.update(&w_grad, &b_grad, learning_rate);

        loss
    }

    pub fn train(&mut self,
                 inputs: &[Tensor<IN>],
                 targets: &[Tensor<OUT>],
                 epochs: usize,
                 learning_rate: f64
    ) -> Vec<f64> {
        assert_eq!(inputs.len(), targets.len(), "Number of inputs and targets must match");
        let mut losses = Vec::with_capacity(epochs);

        for _ in 0..epochs {
            let mut epoch_loss = 0.0;

            for (input, target) in inputs.iter().zip(targets.iter()) {
                epoch_loss += self.train_step(input, target, learning_rate);
            }

            epoch_loss /= inputs.len() as f64;
            losses.push(epoch_loss);
        }

        losses
    }

}

impl<const IN: usize, const HIDDEN: usize, const OUT: usize, const WIDTH: usize> Distribution<NeuralNetwork<IN, HIDDEN, OUT, WIDTH>> for StandardUniform {
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> NeuralNetwork<IN, HIDDEN, OUT, WIDTH> {
        let mut network = NeuralNetwork {
            input: rng.random(),
            hidden: from_fn(|_| rng.random()),
            output: rng.random(),
        };
        network.set_default_activation();
        network
    }
}

pub type TetrisNeuralNetwork = NeuralNetwork<20, 2, 1, 20>;

pub const NEURAL_GENOME_SIZE: usize = TetrisNeuralNetwork::TOTAL_SIZE;
pub type NeuralGenome = Genome<NEURAL_GENOME_SIZE>;

impl Into<NeuralGenome> for TetrisNeuralNetwork {
    fn into(self) -> NeuralGenome {
        let array: [f64; NEURAL_GENOME_SIZE] = self.flatten().try_into().unwrap();
        array.into()
    }
}

impl From<NeuralGenome> for TetrisNeuralNetwork {
    fn from(genome: NeuralGenome) -> Self {
        Self::from_slice(&genome.chromosome().map(Coefficient::into_f64))
    }
}

impl TetrisNeuralNetwork {
    pub fn new(weights: &[f64; NEURAL_GENOME_SIZE]) -> Self {
        let mut network = TetrisNeuralNetwork::from_slice(weights);
        network.set_default_activation();
        network
    }
}

impl Default for TetrisNeuralNetwork {
    fn default() -> Self {
        Self::new(&[0.453658, -0.751826, 0.229369, 0.770971, -0.139467, 0.358945, -0.444327, -0.043037, 0.415310, 0.243784, 0.534104, -0.613669, 0.739624, 0.807618, 0.704872, -0.325011, -0.944927, 0.824014, 0.864859, -0.617375, -0.236017, -0.128543, -0.583322, 0.188471, -0.444104, -0.286296, -0.518049, -0.033349, -0.654863, -0.852937, -0.396723, 0.679449, -0.726166, 0.697776, 0.641854, -0.286143, 0.032658, -0.483823, -0.374967, -0.628506, 0.270259, 0.696688, -0.472999, -0.407123, -1.186373, 0.520987, 0.552181, 0.331578, 0.349316, -0.790625, 0.676089, -0.005859, 0.514797, 0.500138, 0.350346, -0.313741, 0.265229, 0.350491, 0.731158, 0.251483, -0.480060, -0.330814, -0.021579, -0.539752, 0.718832, 0.274903, -0.210526, -0.108341, 0.252360, -0.960174, -0.887229, -0.450100, -0.893969, -1.072814, -0.077854, 0.273594, -0.646591, -0.779650, -0.607125, -0.235472, 0.563436, -0.963237, -0.595069, 0.859383, 0.255186, 0.826308, -0.060939, 0.707190, -0.064852, 0.445402, 0.944277, 0.796731, 0.994174, 0.319467, -0.027827, 1.002416, 0.649488, 0.952832, 0.124481, 0.646331, -0.691047, -0.275365, 0.874940, 0.030911, -0.583123, -0.667755, -0.514440, 0.619087, 0.516929, 0.262956, 0.072757, 0.182821, -0.931387, 0.904671, 0.659955, 1.026166, 0.720739, -0.011151, 0.261878, 0.326589, -0.079616, 0.506177, -0.447848, 0.035897, 1.002810, -0.742900, -0.174787, 0.182951, 0.484245, 0.208084, 0.673592, -0.644417, 0.681093, -0.417648, -0.797382, -0.110807, -0.438872, -0.685694, -0.886660, 0.660114, 0.950884, -0.335964, -0.920430, -0.043131, 0.081588, -0.518454, 0.808717, -0.984122, -0.639347, -0.421940, 0.475266, -0.134133, -0.234250, 1.247525, -0.483487, -0.648677, 0.771206, 0.583906, 0.911886, 0.166647, 0.690531, 0.399068, 0.004320, -0.232041, 0.543498, 0.540038, 0.257154, 0.980085, -0.692713, -0.109042, 0.724863, 1.109307, 0.388879, 0.410072, -0.111650, 0.251050, 0.507149, 0.570829, 0.466856, 0.813850, -0.578225, -0.485365, 0.225000, 0.209530, 0.293852, -0.010099, -0.110912, 0.825857, -0.302769, -0.454535, 0.672503, 0.372170, -0.683253, 0.511142, -0.515754, 0.986362, -0.550108, 0.821692, 0.957912, -0.157110, -0.957272, 0.031299, 0.575343, -0.950626, -0.237092, 0.163650, 0.589844, -0.813365, 0.635152, 0.113734, 0.943986, -0.862969, 0.128938, -0.374695, 0.821890, 0.427037, -0.560447, -0.887806, 0.252297, -0.355189, -0.896571, 0.943943, -0.665350, 0.373945, 0.797277, 0.303691, -0.652086, 0.670329, -0.636550, -0.139600, 0.690017, 0.781238, -0.405011, 0.036049, 0.923967, 0.749273, -0.006155, 0.361679, 0.405863, -0.321033, 0.155170, -0.582898, 0.773287, 0.691835, -0.682684, -0.083123, -0.808344, -0.982037, 0.219689, -0.808301, -0.785741, -0.907759, 0.108219, 1.139180, 0.078296, 0.333608, 0.469053, 0.555608, -0.941535, -0.044189, 0.226595, 0.229060, 0.401641, -0.509846, -0.726063, 0.060460, 0.706651, 0.539763, -0.899528, 0.110490, -0.451073, 0.826044, 0.105147, 0.806962, 0.813589, -0.267172, -0.363821, 0.612839, 0.494354, -0.786546, 0.940971, 0.831415, 0.903213, -0.249233, -0.644508, -0.823586, 0.342659, 0.550663, -1.361540, 0.126367, -0.587526, 0.772548, 0.331339, 0.286757, 0.002213, 0.446865, 0.238464, -0.149814, -0.466978, -0.857801, 2.148990, 3.120307, 0.212056, -0.146238, 0.722892, -0.009014, -0.929592, 0.260544, -0.176071, 0.860693, 0.178414, -0.319288, -0.705731, -0.319909, -0.052305, 1.426165, -0.002815, 0.176373, 0.378439, 0.970022, -0.666771, -0.205879, 0.713923, 0.803402, -0.345752, -0.937211, -0.430338, -0.105874, -0.821215, 0.782624, 0.025377, 0.480402, -0.086370, 0.933525, 0.009236, -0.785254, 0.262059, -0.442774, 0.799184, -0.509999, -0.126970, -0.215653, 0.849435, -0.358330, -0.211699, -0.382208, -0.413333, -0.859224, 0.066229, 0.118595, 0.083030, 1.181578, 0.057438, 0.911593, 0.907058, 0.256075, 0.315352, -0.069982, 0.224998, 0.682668, -0.659572, 0.252855, -0.500238, 0.480969, -0.093622, 0.011524, -0.775071, 0.007165, -0.746697, 0.929513, -0.281621, 0.782337, 0.562959, 0.329243, 0.759639, 0.528228, 0.807098, -0.540471, 0.115638, 0.809493, 0.794721, -0.119104, 0.130660, 0.871640, -0.123047, -0.591011, 0.027932, -0.753167, -0.350399, -0.429369, 0.918842, 0.995677, -0.227048, -0.187357, 0.729255, 0.515664, -0.708834, 0.880885, 0.709089, 0.409704, -0.483505, 0.640837, -0.434402, 0.133775, 0.240845, -0.414214, 0.337688, 0.342248, 0.765617, -0.872100, -0.886510, 0.118805, -0.587917, 0.062952, -0.895483, 0.837070, -0.713895, 0.445770, -0.995722, 0.589201, 0.308659, -0.747434, 0.852238, 0.900151, 0.363923, 1.024947, 0.902725, 0.030534, 0.376659, 0.326433, 0.794322, -0.939945, 0.468721, 0.300523, 0.723429, -0.769387, -0.030907, -0.465654, -0.854004, -0.177790, 0.812149, -0.034354, -0.508212, 0.625013, 0.416380, -0.069606, -0.268961, 0.193668, 0.073681, 0.526838, 0.378218, 0.315745, -0.783572, 0.554689, 0.372606, 0.555562, -0.944835, -0.424549, 0.813999, 0.052270, 0.953311, -0.018533, -0.254989, -0.216605, 0.238316, -0.591479, -0.669579, 0.908886, -0.671410, -0.575061, -0.776352, 0.024044, -0.786027, -0.462303, -0.602359, -0.730508, -0.900817, 0.385126, -0.286024, 0.323902, 0.559069, 0.699264, -0.732626, 0.068356, -0.352026, 0.318859, -0.151870, -0.431301, -0.123649, 0.941264, -0.896369, 0.210409, -0.343441, 0.750510, -0.101514, 0.907179, 0.102523, -0.507965, 0.895191, -0.657702, 0.020105, -0.109551, -0.379357, 0.489263, 0.907157, -0.745518, 0.527301, -0.624799, -0.710481, 0.160614, -0.375961, -0.944024, 0.346542, -0.405355, 0.535816, 0.636534, 0.362162, 0.135838, -0.169195, -0.760267, 0.545853, -0.857810, 0.794127, 0.974894, 0.809802, 0.887709, 0.205520, 0.398789, 0.633283, 0.975885, 0.012840, 0.568764, 0.231484, -0.689870, 0.664885, 0.702183, -0.317363, -0.240179, 0.060708, -0.600814, 0.104109, 0.725210, 0.488573, 0.483129, 0.095870, 0.092670, -0.665257, -0.776717, -0.581027, 0.065059, -0.051504, 0.398069, 0.466709, -0.599995, -1.027020, -0.749893, 0.671853, -0.808757, -0.539119, 0.521685, 0.409500, -0.071922, 0.061871, 0.065100, -0.170052, 0.636058, 0.802492, 0.461406, -0.330666, 0.454641, 0.001693, 0.596916, 0.242522, 0.375851, -0.833358, -0.767888, -0.855085, 0.239785, -0.102884, 0.128661, 0.632367, -0.232645, 0.642423, 0.828786, -0.109414, -0.852921, -0.011673, -0.317314, -0.810973, -0.236458, 0.756385, -0.793898, 0.504036, 0.952273, 0.881264, -0.791483, 0.895072, -0.808931, -0.078656, 0.269303, 0.355972, 0.911547, 0.277065, -0.606783, -0.957576, 0.182537, 0.436202, 0.212920, 0.841326, 0.303588, -0.352357, 0.016540, -0.028290, -0.861237, -0.089538, 0.918839, 0.497592, 0.099501, -0.778634, 0.799227, -0.020773, -0.601729, -0.556313, -0.569223, -0.702531, 0.465188, -0.186028, 0.674572, -0.734969, -0.711867, 1.018785, 0.010585, 0.617296, -0.536746, 0.065701, 0.790436, -0.393710, -0.486487, 0.524527, 0.673891, -0.992980, 0.966151, 0.688251, -0.306151, -0.056877, 0.018996, 0.650717, -0.549902, -0.951988, 0.378882, 0.306766, 0.597303, -0.249555, -0.623182, 0.649984, -0.281433, -0.559471, -0.987487, 0.589709, -0.319157, 0.957196, 0.245386, -0.951546, -0.083993, 0.689070, -0.024174, -0.443682, 0.086366, 0.400921, 0.867220, -0.348043, 0.768839, -0.052670, 0.242625, 0.229352, -0.678755, 0.052431, -0.135489, -1.005900, -0.838944, -0.767093, 0.122692, -0.149832, 0.092685, -0.092739, 0.289265, -0.787539, 0.319546, -0.494208, -0.001959, -0.623362, -0.542674, 0.409660, 0.601635, -0.288216, -0.958826, 0.093576, 0.114001, -0.172416, 0.458887, -0.992899, 0.013683, -0.768151, 0.624015, -0.841362, 0.695514, 0.514174, 0.279431, 0.621734, 0.305403, -0.823422, -0.522009, 0.283812, -0.089201, -0.840294, -0.396080, 0.482740, 0.205662, 0.875475, 0.083708, 0.274201, 0.354810, 0.928292, 0.171859, -0.962115, -0.423088, -0.513398, 0.258406, 0.465895, -0.405983, -0.990159, -0.617647, -0.127107, -1.039054, 0.669722, -0.548353, 0.215092, 0.174823, 0.343147, -0.445232, -0.051554, 0.377992, -0.407724, 0.703712, 0.820024, -0.415079, -0.082844, 0.656238, -0.852734, 0.989994, 0.521734, -0.967493, 0.127919, 0.164165, 0.731523, -0.487829, 0.475846, -0.608826, 0.470810, -0.324995, 0.461790, 0.330232, 0.207716, -0.382868, 0.172308, 0.880139, -0.013769, -0.183699, 0.462890, -0.547008, -0.158741, -0.754520, -0.263159, -0.354594, -0.963217, -0.087915, -0.649921, 0.583671, 0.636164, -0.166322, 0.939099, 0.208063, -0.641854, 0.437904, -0.338570, -0.917081, -0.891791, -0.426410, -0.630696, -0.239139, 0.854517, -0.843767, -0.614698, 0.398929, -0.320206, 0.844774, 0.963399, 0.974855, -0.385484, -0.432487, -0.804489, -0.386200, -0.149590, 0.109864, -0.352027, -0.431122, 0.225825, 0.762809, 0.923699, -0.277955, -0.216563, 0.195004, -0.122103, 0.680998, 0.096636, 1.010983, 0.548233, -0.193410, 0.187398, 0.101385, 0.375824, -0.091198, -0.711503, 0.870485, 0.935106, 0.511082, -0.061624, -0.174294, 0.260398, 0.181652, 0.396327, 0.733199, -0.546080, 0.762210, -0.630421, -0.240102, 0.294247, -0.607539, -0.496208, -0.532346, -0.397542, -0.075516, -0.930530, -0.508675, 0.065846, -0.106159, 0.140354, 0.931537, -0.377223, -0.583149, -0.062945, 0.510677, 0.236282, 0.523521, 0.560337, 0.295918, 0.615273, 0.638335, -0.187681, -0.292695, -0.900610, 0.261145, -0.594001, 0.380987, 0.806742, 0.646228, -0.613796, 0.510273, -0.696158, 0.084849, -0.076809, -0.732630, -0.515350, -0.191619, 0.680269, 0.033068, 0.903079, 0.383730, -0.619102, -0.777185, 0.835553, -0.921946, -0.523858, 0.010511, 0.541669, -0.644410, -0.771169, -0.595220, -0.718867, -0.770386, -0.364933, -0.802186, -0.413265, -0.004551, -0.624236, -0.413160, -1.284147, -0.098826, -0.204075, -0.900189, 0.401247, -0.856648, -0.336995, -0.815478, 0.826203, -0.930698, 0.721599, 0.349293, 0.553471, 0.892650, -0.449004, 0.370798, 0.083212, 0.180398, -0.291874, 0.721022, 0.281526, -0.191874, -0.105758, -0.000161, -0.419739, -0.807113, -0.972212, 0.419685, 0.167795, 0.074341, -0.903789, -0.202575, -0.210088, 0.223595, -0.332741, 0.493000, -0.432674, 0.907981, -0.154932, 0.860915, -0.311019, -0.786090, 0.063516, -0.941399, -0.474552, -0.944123, -0.596339, 0.482495, -0.539614, -0.760753, 0.936290, 0.738129, -0.127046, -0.329279, -0.589786, -0.394846, 0.798638, 0.421375, -0.280019, -0.297496, -0.444807, -0.344327, 0.157343, -0.768323, 0.950965, -0.136364, 0.055447, -0.684502, 0.424763, 0.024487, 0.230896, -0.772387, 0.928337, 0.818900, 0.608018, -0.883485, -0.210960, -0.626861, 0.916266, -0.330435, 0.170903, 0.674160, -0.862466, 0.718317, 0.992084, -0.047981, -0.054069, 0.252356, -0.232106, -0.380099, 0.770870, 0.387321, 0.186149, 0.664004, 0.685494, 0.206978, 0.730109, -0.153141, -0.561179, -1.057000, -0.292701, -0.054777, -0.321514, 0.168259, 0.683189, 0.385575, -0.839367, 0.350993, -0.726219, -0.341570, -0.344823, 0.303860, 0.509156, -1.109105, -0.668069, -0.515289, -0.970874, -0.655758, -0.239984, 0.580138, -0.403376, -0.928845, -0.779635, -0.991515, 0.792219, -0.477120, 0.295459, 0.411388, -0.679909, -0.700974, 0.579702, 0.018923, -0.964551, -0.428363, -0.494802, -0.819450, -0.926544, 0.343015, -0.611873, 0.855908, 0.893077, -0.480261, -0.359168, 0.595773, -0.107886, -0.606843, 0.892851, -0.354667, 0.626887, -0.647407, 0.894940, 0.615014, -0.395834, 0.045180, -0.488776, -0.618463, 0.799009, 0.479513, 0.273459, -0.807441, -0.928624, -0.084211, -0.676850, -0.831358, 0.276554, -0.321761, -0.321513, 0.154557, -0.317265, -0.804186, 0.369525, 0.287458, -0.763131, -0.567686, -0.637202, 0.470616, 0.035885, -1.120516, -0.988977, 0.774595, 0.858863, 0.475412, -0.239941, -0.424423, -0.967942, 0.738425, -0.559498, -0.755260, -0.710056, -0.039934, -0.082726, 0.861747, -0.513857, 0.403021, 0.441532, -0.276450, 0.772144, 0.935270, -0.622343, -0.789190, -0.457964, -0.076714, -0.552576, -0.655826, 0.589745, 0.920704, 0.855316, -0.807591, -0.401398, -0.307433, 0.361896, -0.221582, -0.004115, -0.927141, 0.514417, 0.200940, 0.644069, 0.276483, -0.655377, -0.391788, 0.735383, 0.535361, -0.403696, 0.474769, -0.398578, 0.263385, 0.979038, -0.293863, -0.085846, 0.250889, 0.076982, -0.523387, -0.449696, 0.290288, -0.723427, -0.287692, -0.170435, -0.748164, 0.404286, 0.177138, -1.054276, -0.665138, -0.499061, -0.397299, 0.828156, 0.512477, 0.740448, -0.103924, 0.638818, -0.606245, -0.716313, 0.505426, -0.818136, -0.811289, -0.170550, 0.398426, -0.116281, -0.742978, -0.524063, 0.290339, -1.051292, -0.195184, -0.855091, -0.503805, 0.808216, 0.752381, 0.657945, -0.001671, 0.016725, 0.487266, -0.664082, -0.954157, -0.433052, -0.533743, -0.577312, -0.288337, -0.838335, 0.404689, 0.256219, -0.470361, 0.306077, -0.251195, 0.667857, -0.290804, -0.622587, 0.842690, 0.419264, -0.956381, 0.104729, 0.071438, -0.034484, 0.105739, 0.747021, -0.902699, 0.126385, 0.064203, -0.312460, -0.453841, -0.407390, -0.338243, 0.002424, -0.562563, 0.371306, 0.435610, -0.031409, -0.268388, 0.700765, 0.760494, 0.325954, -0.655126, 1.347278, 0.886011, -0.179079, 0.753731, -0.427112, -0.761291, 0.074251, 1.076434, -0.932717, 0.736752, 0.689388, -0.985286, 0.208325, 0.353615, -0.740018, 0.550503, -0.902716, -0.408059, 0.649925, 0.203061, 0.951645, 0.343151, 0.833279, -0.237570, 0.246832, -0.261240, 0.460678, 0.370840, 0.170136, 0.520920, -0.841211, 0.557487, 0.056541, 0.328982, 0.497416, 0.810037, -0.233904, -0.011109, -0.061672, -0.611143, -0.296125, 0.693375, -0.332148, -0.286769, 0.894517, 0.056154, 0.936613, 0.298346, -0.403940, -0.628225, 0.432350, -0.078285, 0.431465, 0.728198, 0.776217, 0.456294, -0.373556, 0.517354, 0.134814, 1.158041, 0.192358, -0.693254, 0.992942, 0.215090, 1.014121, -0.645994, 0.315434, 0.149837, -0.991666, -0.680721, -0.415797, -0.295440, -0.785107, 0.257991, -0.596720])
    }
}

#[cfg(test)]
mod tests {
    use approx::assert_relative_eq;
    use rand::SeedableRng;
    use rand_chacha::ChaChaRng;
    use super::*;

    #[test]
    fn flatten_tensor() {
        let tensor = Tensor::new([[1., 2., 3.], [4., 5., 6.]]);
        let flat = tensor.flatten();
        let from_flat = Tensor::from_slice(&flat);
        assert_eq!(tensor, from_flat);
    }

    #[test]
    fn flatten_layer() {
        let layer = Layer::fully_connected(
            Tensor::new([[1., 2., 3.], [4., 5., 6.]]),
            Tensor::new([[1.], [2.]]),
            ActivationFunction::default(),
        );
        let flat = layer.flatten();
        let from_flat = Layer::from_slice(&flat);
        assert_eq!(layer, from_flat);
    }

    #[test]
    fn flatten_network() {
        let network = NeuralNetwork {
            input: Layer::fully_connected(
                Tensor::new([[1., 2., 3.], [4., 5., 6.]]),
                Tensor::new([[0.1], [0.2]]),
                ActivationFunction::default(),
            ),
            hidden: [
                Layer::fully_connected(
                    Tensor::new([[7., 8.], [9., 10.]]),
                    Tensor::new([[0.3], [0.4]]),
                    ActivationFunction::default(),
                ),
                Layer::fully_connected(
                    Tensor::new([[11., 12.], [13., 14.]]),
                    Tensor::new([[0.5], [0.6]]),
                    ActivationFunction::default(),
                )
            ],
            output: Layer::fully_connected(
                Tensor::new([[15., 16.]]),
                Tensor::new([[0.7]]),
                ActivationFunction::default(),
            )
        };
        let flattened = network.flatten();
        let expected = vec![
            1., 2., 3., 4., 5., 6., 0.1, 0.2, // input
            7., 8., 9., 10., 0.3, 0.4, // hidden 1
            11., 12., 13., 14., 0.5, 0.6, // hidden 2
            15., 16., 0.7 // output
        ];
        assert_eq!(flattened, expected);

        // Reconstruct network from flattened vector
        let reconstructed = NeuralNetwork::from_slice(&flattened);
        assert_eq!(reconstructed, network);
    }

    #[test]
    fn parse_tetris_network() {
        let weights: [f64; TetrisNeuralNetwork::TOTAL_SIZE] = rand::random();
        let network = TetrisNeuralNetwork::new(&weights);
        let flattened = network.flatten();
        assert_eq!(flattened, weights);
    }

    #[test]
    fn deterministic() {
        let network = TetrisNeuralNetwork::default();
        let mut rng = ChaChaRng::seed_from_u64(42);
        for _ in 0..10 {
            let input = Tensor::vector(rng.random());
            let expected = network.forward(&input).value();
            assert!(!expected.is_nan());
            for _ in 0..10 {
                let result = network.forward(&input).value();
                assert_relative_eq!(result, expected, epsilon = 1e-8);
            }
        }
    }

    #[test]
    fn dot_product() {
        let t1 = Tensor::new([[1., 2., 3.], [4., 5., 6.]]);
        let t2 = Tensor::new([[7., 8.], [9., 10.], [11., 12.]]);
        let result = t1.dot(&t2);
        assert_eq!(result, Tensor::new([[58., 64.], [139., 154.]]));
    }

    #[test]
    fn relu() {
        let mut result = Tensor::new([[-1., 2., 3.], [4., -5., 6.]]);
        result.relu_mut();
        assert_eq!(result, Tensor::new([[0., 2., 3.], [4., 0., 6.]]));
    }

    #[test]
    fn add() {
        let t1 = Tensor::new([[1., 2., 3.], [4., 5., 6.]]);
        let t2 = Tensor::new([[7., 8., 9.], [10., 11., 12.]]);
        let result = t1 + t2;
        assert_eq!(result, Tensor::new([[8., 10., 12.], [14., 16., 18.]]));
    }

    #[test]
    fn fully_connected_layer_forward() {
        let layer = Layer::fully_connected(
            Tensor::new([[1., 2., 3.], [4., 5., 6.]]),
            Tensor::new([[1.], [2.]]),
            ActivationFunction::ReLU,
        );

        let ones = Tensor::ONES;
        let observed = layer.forward(&ones);
        assert_eq!(observed, Tensor::vector([7., 17.]));
    }

    #[test]
    fn test_mcculloch_pitt_network() {
        // network from https://blog.abhranil.net/2015/03/03/training-neural-networks-with-genetic-algorithms/
        let network: NeuralNetwork<2, 0, 1, 2> = NeuralNetwork {
            input: Layer::mcculloch_pitt(
                Tensor::new([[1.0, 1.0], [-1.0, -1.0]]),
                [0.5,-1.5]
            ),
            hidden: [],
            output: Layer::mcculloch_pitt(
                Tensor::new([[1.0, 1.0]]),
                [1.5],
            ),
        };

        for x in [0, 1] {
            for y in [0, 1] {
                let expected = if x == y { 0.0 } else { 1.0 };
                let observed = network.forward(&Tensor::vector([x as f64, y as f64]));
                assert_eq!(observed.value(), expected, "x={}, y={}", x, y);
            }
        }
    }

    #[test]
    fn test_train_x_plus_y() {
        let mut rng = ChaChaRng::seed_from_u64(100);
        let network = train_network::<0, 2>(&mut rng, 100, 1500, |x, y| x + y);
        validate_network(&mut rng, network, 100, |x, y| x + y);
    }

    #[test]
    fn test_train_x_mul_y() {
        let mut rng = ChaChaRng::seed_from_u64(100);
        let network = train_network::<0, 8>(&mut rng, 500, 5000, |x, y| x * y);
        validate_network(&mut rng, network, 100, |x, y| x * y);
    }

    fn random_xy(rng: &mut ChaChaRng) -> (f64, f64) {
        let x = rng.random_range(0. .. 1.);
        let y = rng.random_range(0. .. 1.);
        (x, y)
    }

    fn train_network<const HIDDEN: usize, const WIDTH: usize>(
        rng: &mut ChaChaRng,
        training_set_size: usize,
        epochs: usize,
        function: impl Fn(f64, f64) -> f64
    ) -> NeuralNetwork<2, HIDDEN, 1, WIDTH> {
        // Create a simple network: 2 inputs, 1 output
        let mut network: NeuralNetwork<2, HIDDEN, 1, WIDTH> = rng.random();
        network.set_activation(ActivationFunction::Sigmoid);


        // build training data from random numbers
        let mut inputs = vec![];
        let mut targets = vec![];
        for _ in 0..training_set_size {
            let (x, y) = random_xy(rng);
            inputs.push(Tensor::vector([x, y]));
            targets.push(Tensor::vector([function(x, y)]))
        }

        // Train the network
        network.train(&inputs, &targets, epochs, 0.01);

        network
    }

    fn validate_network<const HIDDEN: usize, const WIDTH: usize>(
        rng: &mut ChaChaRng,
        network: NeuralNetwork<2, HIDDEN, 1, WIDTH>,
        validation_set_size: usize,
        function: impl Fn(f64, f64) -> f64
    ) {
        let mut sum_error = 0.0;
        for _ in 0..validation_set_size {
            let (x, y) = random_xy(rng);
            let expected = function(x, y);
            let observed = network.forward(&Tensor::vector([x, y]));
            sum_error += (expected - observed.value()).abs();
        }

        let mean_error = sum_error / validation_set_size as f64;
        assert_relative_eq!(
            mean_error,
            0.0,
            epsilon = 0.01, // within 1%
        );
    }

}
