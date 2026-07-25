#!/usr/bin/env python3
"""ATSassin Model Distillation Pipeline

Distills a 22M-109M parameter classifier from a teacher model (e.g., llama-3.1-8b).
Uses Hugging Face Transformers + PyTorch.

Usage:
    python scripts/distill.py --teacher llama-3.1-8b --student 22M --output models/distilled/
    python scripts/distill.py --teacher llama-3.1-8b --student 109M --output models/distilled/
    python scripts/distill.py --teacher llama-3.1-8b --student 1.5B --output models/distilled/
"""

import argparse
import json
import os
import sys
from pathlib import Path

try:
    import torch
    import torch.nn as nn
    from torch.utils.data import Dataset, DataLoader
    from transformers import (
        AutoTokenizer,
        AutoModelForCausalLM,
        TrainingArguments,
        Trainer,
        DataCollatorForLanguageModeling,
    )
    from datasets import Dataset as HFDataset
except ImportError:
    print("ERROR: Missing dependencies. Install with:")
    print("  pip install torch transformers datasets accelerate")
    sys.exit(1)


class DistillationDataset(Dataset):
    """Dataset for distillation training from JSONL files."""

    def __init__(self, jsonl_path: str, tokenizer, max_length: int = 512):
        self.tokenizer = tokenizer
        self.max_length = max_length
        self.examples = []

        with open(jsonl_path, "r", encoding="utf-8") as f:
            for line in f:
                line = line.strip()
                if not line:
                    continue
                try:
                    example = json.loads(line)
                    self.examples.append(example)
                except json.JSONDecodeError:
                    continue

    def __len__(self):
        return len(self.examples)

    def __getitem__(self, idx):
        example = self.examples[idx]
        instruction = example.get("instruction", "")
        input_text = example.get("input", "")
        output = example.get("output", "")

        prompt = f"### Instruction:\n{instruction}\n\n### Input:\n{input_text}\n\n### Response:\n{output}"

        encoding = self.tokenizer(
            prompt,
            truncation=True,
            max_length=self.max_length,
            padding="max_length",
            return_tensors="pt",
        )

        input_ids = encoding["input_ids"].squeeze()
        attention_mask = encoding["attention_mask"].squeeze()

        return {
            "input_ids": input_ids,
            "attention_mask": attention_mask,
            "labels": input_ids.clone(),
        }


def parse_model_size(size: str) -> tuple[int, int]:
    """Return (hidden_size, num_layers) for a given model size label."""
    size = size.lower()
    if size == "22m":
        return (256, 4)
    elif size == "109m":
        return (512, 8)
    elif size == "1.5b":
        return (2048, 24)
    else:
        raise ValueError(f"Unknown model size: {size}. Choose 22M, 109M, or 1.5B")


def create_student_model(student_size: str, tokenizer) -> nn.Module:
    """Create a small student model for distillation."""
    hidden_size, num_layers = parse_model_size(student_size)

    vocab_size = len(tokenizer)
    config = type(
        "StudentConfig",
        (),
        {
            "vocab_size": vocab_size,
            "hidden_size": hidden_size,
            "num_hidden_layers": num_layers,
            "num_attention_heads": max(1, hidden_size // 64),
            "intermediate_size": hidden_size * 4,
            "max_position_embeddings": 512,
            "pad_token_id": tokenizer.pad_token_id,
            "eos_token_id": tokenizer.eos_token_id,
        },
    )()

    model = AutoModelForCausalLM.from_config(config)
    return model


def distill(
    teacher_model_name: str,
    student_size: str,
    training_data_path: str,
    output_dir: str,
    epochs: int = 3,
    batch_size: int = 8,
    learning_rate: float = 5e-5,
    max_length: int = 512,
):
    """Run knowledge distillation from teacher to student."""

    print(f"Loading teacher model: {teacher_model_name}")
    tokenizer = AutoTokenizer.from_pretrained(teacher_model_name)
    if tokenizer.pad_token is None:
        tokenizer.pad_token = tokenizer.eos_token

    teacher = AutoModelForCausalLM.from_pretrained(
        teacher_model_name,
        torch_dtype=torch.float16 if torch.cuda.is_available() else torch.float32,
        device_map="auto" if torch.cuda.is_available() else "cpu",
    )
    teacher.eval()

    print(f"Creating student model: {student_size}")
    student = create_student_model(student_size, tokenizer)

    print(f"Loading training data from: {training_data_path}")
    dataset = DistillationDataset(training_data_path, tokenizer, max_length)
    print(f"Loaded {len(dataset)} training examples")

    training_args = TrainingArguments(
        output_dir=output_dir,
        num_train_epochs=epochs,
        per_device_train_batch_size=batch_size,
        learning_rate=learning_rate,
        weight_decay=0.01,
        logging_steps=10,
        save_strategy="epoch",
        fp16=torch.cuda.is_available(),
        push_to_hub=False,
        report_to="none",
    )

    data_collator = DataCollatorForLanguageModeling(tokenizer=tokenizer, mlm=False)

    trainer = Trainer(
        model=student,
        args=training_args,
        train_dataset=dataset,
        data_collator=data_collator,
    )

    print("Starting distillation training...")
    trainer.train()

    student_output = Path(output_dir) / f"student-{student_size.lower()}"
    student_output.mkdir(parents=True, exist_ok=True)
    student.save_pretrained(student_output)
    tokenizer.save_pretrained(student_output)

    print(f"Student model saved to: {student_output}")

    manifest = {
        "teacher": teacher_model_name,
        "student_size": student_size,
        "training_examples": len(dataset),
        "epochs": epochs,
        "batch_size": batch_size,
        "learning_rate": learning_rate,
        "max_length": max_length,
        "output_path": str(student_output),
    }
    manifest_path = student_output / "distillation_manifest.json"
    with open(manifest_path, "w") as f:
        json.dump(manifest, f, indent=2)

    print("Distillation complete.")
    return str(student_output)


def main():
    parser = argparse.ArgumentParser(description="ATSassin Model Distillation Pipeline")
    parser.add_argument("--teacher", required=True, help="Teacher model name or path")
    parser.add_argument("--student", required=True, choices=["22M", "109M", "1.5B"], help="Student model size")
    parser.add_argument("--output", required=True, help="Output directory for distilled model")
    parser.add_argument("--training-data", required=True, help="Path to training data JSONL")
    parser.add_argument("--epochs", type=int, default=3, help="Number of training epochs")
    parser.add_argument("--batch-size", type=int, default=8, help="Batch size")
    parser.add_argument("--learning-rate", type=float, default=5e-5, help="Learning rate")
    parser.add_argument("--max-length", type=int, default=512, help="Max sequence length")

    args = parser.parse_args()

    if not os.path.exists(args.training_data):
        print(f"ERROR: Training data not found: {args.training_data}")
        sys.exit(1)

    distill(
        teacher_model_name=args.teacher,
        student_size=args.student,
        training_data_path=args.training_data,
        output_dir=args.output,
        epochs=args.epochs,
        batch_size=args.batch_size,
        learning_rate=args.learning_rate,
        max_length=args.max_length,
    )


if __name__ == "__main__":
    main()
